//! Windows mode, high contrast and its colours.
//!
//! Owner: `p5-theme`.
//!
//! The icon and the menu follow the Windows mode (`SystemUsesLightTheme`, the taskbar's), not the app mode (product
//! decision 3). Every read goes to the registry and `GetSysColor`, never to uxtheme's cached state, which lagged a
//! switch in Phase 0 (`docs/research.md`, "Live theme switch through Settings").

use std::ffi::c_void;

use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR,
};
use windows::Win32::Graphics::Gdi::{
    COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT,
    COLOR_HOTLIGHT, COLOR_WINDOW, COLOR_WINDOWTEXT, GetSysColor, SYS_COLOR_INDEX,
};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};
use windows::core::w;

use crate::WinError;

/// A colour as red, green and blue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// From a GDI `COLORREF` (`0x00BBGGRR`), as `GetSysColor` returns it.
    pub const fn from_colorref(c: u32) -> Self {
        Self::new(c as u8, (c >> 8) as u8, (c >> 16) as u8)
    }

    /// As a GDI `COLORREF` (`0x00BBGGRR`).
    pub const fn colorref(self) -> u32 {
        self.r as u32 | (self.g as u32) << 8 | (self.b as u32) << 16
    }
}

impl From<Rgb> for [u8; 3] {
    fn from(c: Rgb) -> Self {
        [c.r, c.g, c.b]
    }
}

/// What the icon and the menu draw with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    /// `SystemUsesLightTheme`: the taskbar's mode, which Windows' own tray menus follow.
    pub light: bool,
    /// `Some` only while high contrast is on; then the colours below replace every light and dark colour.
    pub contrast: Option<Contrast>,
}

/// The high contrast system colours the icon art and the menu need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contrast {
    pub window: Rgb,
    pub window_text: Rgb,
    pub highlight: Rgb,
    pub highlight_text: Rgb,
    pub gray_text: Rgb,
    pub button_face: Rgb,
    pub button_text: Rgb,
    pub hot_light: Rgb,
}

/// The mode Windows 11 installs with, used when `SystemUsesLightTheme` is missing: `InstallTheme` is `aero.theme`,
/// whose `SystemMode` is Light (`docs/research.md`, "Tray UX: theme reads"). What Explorer itself does with a missing
/// value is not documented.
const LIGHT_WHEN_MISSING: bool = true;

/// One read of the Windows mode, high contrast and its colours. A failed read is an error, never a guess.
pub fn read() -> Result<Theme, WinError> {
    let light = read_light()?;
    let on = high_contrast_on()?;
    // SAFETY: takes an index and returns a value; an unknown index gives 0, and every index here is documented.
    let contrast = contrast_from(on, |i| unsafe { GetSysColor(i) });
    Ok(Theme { light, contrast })
}

/// Reports a theme only when it differs from the last one, because "ImmersiveColorSet" comes twice per switch (the
/// first time with stale values) and also with no change at all (`docs/research.md`).
#[derive(Clone, Copy, Debug)]
pub struct ThemeWatch {
    last: Theme,
}

impl ThemeWatch {
    pub fn new(first: Theme) -> Self {
        Self { last: first }
    }

    /// The theme of the last read that worked, or the first one.
    pub fn current(&self) -> Theme {
        self.last
    }

    /// Reads now; `Some` with the new theme only when it changed.
    pub fn check(&mut self) -> Option<Theme> {
        self.check_with(read)
    }

    /// As `check`, with the read given. A failed read keeps the last theme, so one bad read never flips the icon.
    pub fn check_with(&mut self, read: impl FnOnce() -> Result<Theme, WinError>) -> Option<Theme> {
        let now = read().ok()?;
        if now == self.last {
            return None;
        }
        self.last = now;
        Some(now)
    }
}

const PERSONALIZE: windows::core::PCWSTR =
    w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");

fn read_light() -> Result<bool, WinError> {
    let mut data = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: the names are static NUL-terminated strings; `data` and `size` are live locals of the sizes given.
    let code = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PERSONALIZE,
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut data).cast::<c_void>()),
            Some(&raw mut size),
        )
    };
    light_from(code, data)
}

/// A missing value or key is Windows' default; any other failure, a value of another type included, is an error.
fn light_from(code: WIN32_ERROR, data: u32) -> Result<bool, WinError> {
    match code {
        ERROR_SUCCESS => Ok(data != 0),
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(LIGHT_WHEN_MISSING),
        other => Err(WinError {
            call: "RegGetValueW",
            code: other.0,
        }),
    }
}

fn high_contrast_on() -> Result<bool, WinError> {
    let mut hc = HIGHCONTRASTW {
        cbSize: size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    // SAFETY: `hc` is a live HIGHCONTRASTW with `cbSize` set, as `SPI_GETHIGHCONTRAST` requires; the scheme name
    // pointer Windows writes into it is never read.
    unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            hc.cbSize,
            Some((&raw mut hc).cast::<c_void>()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .map_err(|e| win_error("SystemParametersInfoW", &e))?;
    Ok(hc.dwFlags.contains(HCF_HIGHCONTRASTON))
}

/// Colours are read only with high contrast on: outside it they are the classic theme's and mean nothing here.
fn contrast_from(on: bool, mut colour: impl FnMut(SYS_COLOR_INDEX) -> u32) -> Option<Contrast> {
    if !on {
        return None;
    }
    let mut c = |i| Rgb::from_colorref(colour(i));
    Some(Contrast {
        window: c(COLOR_WINDOW),
        window_text: c(COLOR_WINDOWTEXT),
        highlight: c(COLOR_HIGHLIGHT),
        highlight_text: c(COLOR_HIGHLIGHTTEXT),
        gray_text: c(COLOR_GRAYTEXT),
        button_face: c(COLOR_BTNFACE),
        button_text: c(COLOR_BTNTEXT),
        hot_light: c(COLOR_HOTLIGHT),
    })
}

/// Win32 errors keep their Win32 code, so a log line matches the docs; other failures keep the HRESULT.
fn win_error(call: &'static str, e: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: WIN32_ERROR::from_error(e).map_or(e.code().0 as u32, |w| w.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_UNSUPPORTED_TYPE};

    const DARK: Theme = Theme {
        light: false,
        contrast: None,
    };
    const LIGHT: Theme = Theme {
        light: true,
        contrast: None,
    };

    /// Each colour encodes its own index, so a test sees which index filled which field.
    fn by_index(i: SYS_COLOR_INDEX) -> u32 {
        i.0 as u32 * 0x0001_0101
    }

    fn contrast() -> Contrast {
        contrast_from(true, by_index).expect("contrast on gives colours")
    }

    fn failed() -> Result<Theme, WinError> {
        Err(WinError {
            call: "RegGetValueW",
            code: ERROR_ACCESS_DENIED.0,
        })
    }

    #[test]
    fn watch_reports_only_changes() {
        let mut watch = ThemeWatch::new(DARK);
        assert_eq!(
            watch.check_with(|| Ok(DARK)),
            None,
            "the same values are no change"
        );
        assert_eq!(watch.check_with(|| Ok(LIGHT)), Some(LIGHT));
        assert_eq!(
            watch.check_with(|| Ok(LIGHT)),
            None,
            "the second broadcast of one switch"
        );
        assert_eq!(watch.check_with(|| Ok(DARK)), Some(DARK));
        assert_eq!(watch.current(), DARK);
    }

    #[test]
    fn watch_keeps_last_on_read_failure() {
        let mut watch = ThemeWatch::new(LIGHT);
        assert_eq!(watch.check_with(failed), None);
        assert_eq!(watch.current(), LIGHT);
        assert_eq!(
            watch.check_with(|| Ok(LIGHT)),
            None,
            "the last theme was kept, so this is no change"
        );
        assert_eq!(watch.check_with(|| Ok(DARK)), Some(DARK));
    }

    #[test]
    fn watch_reports_contrast_on_and_off() {
        let on = Theme {
            light: false,
            contrast: Some(contrast()),
        };
        let mut watch = ThemeWatch::new(DARK);
        assert_eq!(watch.check_with(|| Ok(on)), Some(on));
        assert_eq!(watch.check_with(|| Ok(on)), None);

        // Another contrast theme keeps the mode and the flag but changes the colours.
        let mut other = on;
        if let Some(c) = other.contrast.as_mut() {
            c.window = Rgb::new(0xff, 0xff, 0xff);
        }
        assert_eq!(watch.check_with(|| Ok(other)), Some(other));

        assert_eq!(watch.check_with(|| Ok(DARK)), Some(DARK));
        assert_eq!(watch.current().contrast, None);
    }

    #[test]
    fn contrast_colours_only_with_contrast_on() {
        let mut reads = 0;
        let off = contrast_from(false, |i| {
            reads += 1;
            by_index(i)
        });
        assert_eq!(off, None);
        assert_eq!(reads, 0, "no colour is read with contrast off");

        let c = contrast();
        let index = |rgb: Rgb| i32::from(rgb.r);
        assert_eq!(index(c.window), COLOR_WINDOW.0);
        assert_eq!(index(c.window_text), COLOR_WINDOWTEXT.0);
        assert_eq!(index(c.highlight), COLOR_HIGHLIGHT.0);
        assert_eq!(index(c.highlight_text), COLOR_HIGHLIGHTTEXT.0);
        assert_eq!(index(c.gray_text), COLOR_GRAYTEXT.0);
        assert_eq!(index(c.button_face), COLOR_BTNFACE.0);
        assert_eq!(index(c.button_text), COLOR_BTNTEXT.0);
        assert_eq!(index(c.hot_light), COLOR_HOTLIGHT.0);
    }

    #[test]
    fn colorref_is_red_in_the_low_byte() {
        let c = Rgb::from_colorref(0x00_33_22_11);
        assert_eq!(c, Rgb::new(0x11, 0x22, 0x33));
        assert_eq!(c.colorref(), 0x00_33_22_11);
        assert_eq!(<[u8; 3]>::from(c), [0x11, 0x22, 0x33]);
    }

    #[test]
    fn missing_value_reads_as_the_windows_11_default() {
        assert_eq!(light_from(ERROR_SUCCESS, 1), Ok(true));
        assert_eq!(light_from(ERROR_SUCCESS, 0), Ok(false));
        assert_eq!(light_from(ERROR_FILE_NOT_FOUND, 0), Ok(true));
        assert_eq!(light_from(ERROR_PATH_NOT_FOUND, 0), Ok(true));
        assert_eq!(
            light_from(ERROR_UNSUPPORTED_TYPE, 0),
            Err(WinError {
                call: "RegGetValueW",
                code: ERROR_UNSUPPORTED_TYPE.0
            }),
            "a value of another type is an error, not a guess"
        );
    }

    #[test]
    fn reads_this_pc() {
        let theme = read();
        println!("theme on this PC: {theme:?}");
        assert!(theme.is_ok(), "{theme:?}");
    }
}
