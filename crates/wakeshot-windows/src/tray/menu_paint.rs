//! Menu layout, colours and item painting, with no window.
//!
//! Owner: `p5-menu-paint`.
//!
//! The drawing half of the owner-drawn menu picked in Phase 1 (`docs/research/tray-menu-look.md`, "Pick"):
//! `p5-menu-view` calls [`measure`] from `WM_MEASUREITEM` and [`paint_item`] from `WM_DRAWITEM`. Sizes and colours
//! are the WinUI 3 `MenuFlyout` template's (microsoft-ui-xaml `MenuFlyout_themeresources.xaml`), so the menu matches
//! the XAML menus of Explorer and the taskbar, while Windows keeps the menu loop, keyboard and screen readers.

use std::cell::RefCell;
use std::ffi::c_void;

use wakeshot_core::text;
use wakeshot_core::view::{MenuCommand, MenuItem};
use windows::Win32::Foundation::{
    COLORREF, ERROR_NOT_ENOUGH_MEMORY, ERROR_NOT_FOUND, GetLastError, RECT, SIZE, WIN32_ERROR,
};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleDC, CreateDIBSection,
    CreateFontIndirectW, DEFAULT_CHARSET, DEFAULT_QUALITY, DIB_RGB_COLORS, DRAW_TEXT_FORMAT,
    DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS, DT_HIDEPREFIX, DT_LEFT, DT_NOCLIP, DT_NOPREFIX,
    DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, FW_NORMAL,
    GGI_MARK_NONEXISTING_GLYPHS, GdiFlush, GetGlyphIndicesW, GetTextExtentPoint32W, GetTextFaceW,
    HBITMAP, HDC, HFONT, HGDIOBJ, LOGFONTW, SRCCOPY, SelectObject, SetBkMode, SetTextColor,
    TRANSPARENT,
};
use windows::Win32::UI::Controls::{ODS_NOACCEL, ODS_SELECTED};
use windows::Win32::UI::HiDpi::SystemParametersInfoForDpi;
use windows::Win32::UI::WindowsAndMessaging::{NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS};
use windows::core::PCWSTR;

use crate::WinError;
use crate::tray::theme::{Rgb, Theme};

// Effective pixels (epx, 96 dpi) from the WinUI 3 template, with the narrow padding WinUI uses when the menu was
// opened with mouse, pen or keyboard (`MenuFlyoutItemThemePaddingNarrow` 11,4,11,5).
const MARGIN_X: i32 = 4;
const MARGIN_Y: i32 = 2;
const PAD_X: i32 = 11;
const PAD_TOP: i32 = 4;
const PAD_BOTTOM: i32 = 5;
const LINE: i32 = 20;
/// `MenuFlyoutItemPlaceholderThemeThickness`: one column for a mark or an icon; both make it 56.
const COLUMN: i32 = 28;
const MARK: i32 = 12;
const ICON: i32 = 16;
const CHEVRON: i32 = 12;
/// `MenuFlyoutItemChevronMargin`: the gap between the text and the chevron.
const CHEVRON_GAP: i32 = 24;
const SEPARATOR_PAD: i32 = 1;
const SEPARATOR_LINE: i32 = 1;
const RADIUS: i32 = 4;
const FONT: i32 = 14;
/// The widest text an item asks room for: two hold groups fit, more end in an ellipsis. WinUI sets no limit; ours
/// keeps a long hold reason from stretching the menu across the screen.
const TEXT_MAX: i32 = 480;

const GLYPH_CHECK: u16 = 0xE73E;
const GLYPH_RADIO: u16 = 0xE915;
const GLYPH_CHEVRON: u16 = 0xE974;

// Item icons, Segoe Fluent Icons (the same code points are in Segoe MDL2 Assets): an eye for Automatic, which
// watches agents; a clock for Keep awake, since each of its choices is a length of time (Haik, 2026-11-05: "Recent"
// E823, because "Clock" E917 draws the same clock but is missing from MDL2); the restore arrow; a page for the log;
// the close cross for Quit, since a power button would read as shutting the PC down.
const ICON_AUTOMATIC: u16 = 0xE890;
const ICON_KEEP_AWAKE: u16 = 0xE823;
const ICON_RESTORE: u16 = 0xE777;
const ICON_LOG: u16 = 0xE8A5;
const ICON_QUIT: u16 = 0xE711;

const TEXT_FACE: &str = "Segoe UI Variable Text";
const TEXT_FACE_OLD: &str = "Segoe UI";
const GLYPH_FACE: &str = "Segoe Fluent Icons";
const GLYPH_FACE_OLD: &str = "Segoe MDL2 Assets";
/// Faces for a label the text font cannot draw (hold reasons are user text; Haik's own language is Armenian, which
/// Segoe UI Variable lacks). Segoe UI first, then the Windows UI font of each script group.
const FALLBACK_FACES: [&str; 12] = [
    "Segoe UI",
    "Microsoft YaHei UI",
    "Yu Gothic UI",
    "Malgun Gothic",
    "Microsoft JhengHei UI",
    "Nirmala UI",
    "Leelawadee UI",
    "Ebrima",
    "Gadugi",
    "Segoe UI Historic",
    "Segoe UI Symbol",
    "Segoe UI Emoji",
];

/// `epx` at `dpi`, rounded to the nearest pixel as `MulDiv` rounds, so 33 epx is 41 px at 120 dpi like Phase 1's.
pub fn scale(epx: i32, dpi: u32) -> i32 {
    (epx * dpi as i32 + 48) / 96
}

/// Which columns a menu reserves before the text. WinUI reserves them for every item once any item of the menu has
/// a mark or an icon, so the texts of one menu start at one edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Columns {
    pub marks: bool,
    pub icons: bool,
}

impl Columns {
    pub fn of(items: &[MenuItem]) -> Self {
        Self {
            marks: items.iter().any(|item| match item {
                MenuItem::Action { command, .. } => mark_for(*command).is_some(),
                _ => false,
            }),
            icons: items.iter().any(|item| icon_for(item).is_some()),
        }
    }
}

/// Sizes in physical pixels for one menu at one DPI. The DPI is the menu's monitor's, never the system DPI, which is
/// fixed at sign-in (`docs/research.md`, "`LoadIconMetric` follows the system DPI").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub dpi: u32,
    pub columns: Columns,
    /// Item height: 33 epx.
    pub row: i32,
    /// Separator slot height, and its line's top and thickness inside the slot.
    pub separator: i32,
    pub separator_top: i32,
    pub separator_line: i32,
    /// The hover fill's inset from the item edge, and its corner radius.
    pub inset_x: i32,
    pub inset_y: i32,
    pub radius: i32,
    /// The text line box: top and height.
    pub line_top: i32,
    pub line: i32,
    /// Left edges of the mark, the icon and the text.
    pub mark_left: i32,
    pub icon_left: i32,
    pub text_left: i32,
    /// Space right of the text: padding, and for a submenu the gap and the chevron.
    pub right: i32,
    pub chevron_right: i32,
    /// Glyph and font sizes (em height).
    pub mark: i32,
    pub icon: i32,
    pub chevron: i32,
    pub font: i32,
}

impl Layout {
    pub fn new(dpi: u32, columns: Columns) -> Self {
        let s = |epx| scale(epx, dpi);
        // Every edge is scaled from its own epx sum, so rounding never adds up across columns.
        let first = MARGIN_X + PAD_X;
        let marks = if columns.marks { COLUMN } else { 0 };
        let icons = if columns.icons { COLUMN } else { 0 };
        Self {
            dpi,
            columns,
            row: s(MARGIN_Y + PAD_TOP + LINE + PAD_BOTTOM + MARGIN_Y),
            separator: s(SEPARATOR_PAD + SEPARATOR_LINE + SEPARATOR_PAD),
            separator_top: s(SEPARATOR_PAD),
            separator_line: (s(SEPARATOR_PAD + SEPARATOR_LINE) - s(SEPARATOR_PAD)).max(1),
            inset_x: s(MARGIN_X),
            inset_y: s(MARGIN_Y),
            radius: s(RADIUS),
            line_top: s(MARGIN_Y + PAD_TOP),
            line: s(MARGIN_Y + PAD_TOP + LINE) - s(MARGIN_Y + PAD_TOP),
            mark_left: s(first),
            icon_left: s(first + marks),
            text_left: s(first + marks + icons),
            right: s(PAD_X + MARGIN_X),
            chevron_right: s(CHEVRON_GAP + CHEVRON + PAD_X + MARGIN_X),
            mark: s(MARK),
            icon: s(ICON),
            chevron: s(CHEVRON),
            font: s(FONT),
        }
    }

    /// The width the item needs around a text `text_px` wide.
    pub fn width_for(&self, text_px: i32, submenu: bool) -> i32 {
        let right = if submenu {
            self.chevron_right
        } else {
            self.right
        };
        self.text_left + text_px + right
    }
}

/// The colours of one menu. Light and dark are the WinUI tokens laid over the menu fill (no acrylic behind a classic
/// menu window); high contrast is WinUI's high contrast dictionary in system colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// The menu fill, also for `MENUINFO.hbrBack`.
    pub background: Rgb,
    pub text: Rgb,
    /// Marks at rest, the chevron, the status line.
    pub secondary: Rgb,
    pub disabled: Rgb,
    pub hover: Rgb,
    pub hover_text: Rgb,
    pub hover_secondary: Rgb,
    pub separator: Rgb,
    /// For `DWMWA_BORDER_COLOR` on the menu window (`p5-menu-view`).
    pub border: Rgb,
}

impl Palette {
    pub fn for_theme(theme: &Theme) -> Self {
        if let Some(c) = theme.contrast {
            return Self {
                background: c.window,
                text: c.button_text,
                secondary: c.button_text,
                disabled: c.gray_text,
                hover: c.highlight,
                hover_text: c.highlight_text,
                hover_secondary: c.highlight_text,
                separator: c.window_text,
                border: c.window_text,
            };
        }
        // ARGB tokens from `Common_themeresources_any.xaml`. The fills are the classic menu's own, measured in Phase 1.
        let (background, primary, secondary, disabled, hover, divider, stroke) = if theme.light {
            (
                Rgb::new(0xf9, 0xf9, 0xf9),
                0xE400_0000,
                0x9E00_0000,
                0x5C00_0000,
                0x0900_0000,
                0x0F00_0000,
                0x0F00_0000,
            )
        } else {
            (
                Rgb::new(0x2c, 0x2c, 0x2c),
                0xFFFF_FFFF,
                0xC5FF_FFFF,
                0x5DFF_FFFF,
                0x0FFF_FFFF,
                0x15FF_FFFF,
                0x3300_0000,
            )
        };
        let hover = over(hover, background);
        Self {
            background,
            text: over(primary, background),
            secondary: over(secondary, background),
            disabled: over(disabled, background),
            hover,
            hover_text: over(primary, hover),
            hover_secondary: over(secondary, hover),
            separator: over(divider, background),
            border: over(stroke, background),
        }
    }
}

/// The solid colour an ARGB token gives over `bg`.
fn over(argb: u32, bg: Rgb) -> Rgb {
    let a = (argb >> 24) as f32 / 255.0;
    let mix = |c: u32, b: u8| (c as f32 * a + f32::from(b) * (1.0 - a)).round() as u8;
    Rgb::new(
        mix((argb >> 16) & 0xff, bg.r),
        mix((argb >> 8) & 0xff, bg.g),
        mix(argb & 0xff, bg.b),
    )
}

fn lerp(from: Rgb, to: Rgb, t: f32) -> Rgb {
    let m = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
    Rgb::new(m(from.r, to.r), m(from.g, to.g), m(from.b, to.b))
}

/// A pixel of a 32-bit DIB: `0x00RRGGBB`.
fn pixel(c: Rgb) -> u32 {
    u32::from(c.r) << 16 | u32::from(c.g) << 8 | u32::from(c.b)
}

/// The mark a checked item shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Check,
    Radio,
}

impl Mark {
    pub fn glyph(self) -> u16 {
        match self {
            Mark::Check => GLYPH_CHECK,
            Mark::Radio => GLYPH_RADIO,
        }
    }
}

/// On and off options get a check; the Keep awake lengths are one choice of several, so a bullet.
pub fn mark_for(command: MenuCommand) -> Option<Mark> {
    match command {
        MenuCommand::Automatic(_) | MenuCommand::KeepScreenOn(_) => Some(Mark::Check),
        MenuCommand::KeepAwake(_) => Some(Mark::Radio),
        MenuCommand::Restore | MenuCommand::OpenLog | MenuCommand::Quit => None,
    }
}

/// The icon glyph of an item. Only items with a picture Windows users know get one (Microsoft's menu guidance:
/// "Don't feel obligated to provide icons for commands that don't have a standard visualization"): the Keep awake
/// lengths and Keep screen on are a list of choices with marks, and the status line is not a command.
pub fn icon_for(item: &MenuItem) -> Option<u16> {
    match item {
        MenuItem::Action { command, .. } => match command {
            MenuCommand::Automatic(_) => Some(ICON_AUTOMATIC),
            MenuCommand::Restore => Some(ICON_RESTORE),
            MenuCommand::OpenLog => Some(ICON_LOG),
            MenuCommand::Quit => Some(ICON_QUIT),
            MenuCommand::KeepAwake(_) | MenuCommand::KeepScreenOn(_) => None,
        },
        MenuItem::Submenu { text: t, .. } if *t == text::keep_awake_menu() => Some(ICON_KEEP_AWAKE),
        MenuItem::Submenu { .. } | MenuItem::Label(_) | MenuItem::Separator => None,
    }
}

/// What `WM_DRAWITEM` says about an item besides the model: hot, and whether Windows shows access keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemState {
    /// `ODS_SELECTED`: hot under the mouse or the keyboard, or a submenu that is open.
    pub selected: bool,
    /// `ODS_NOACCEL` off: Windows asks for keyboard cues, so access keys are underlined.
    pub keyboard_cues: bool,
}

impl ItemState {
    /// From `DRAWITEMSTRUCT.itemState`. Checked and enabled come from the model, which the menu was built from.
    pub fn from_ods(item_state: u32) -> Self {
        Self {
            selected: item_state & ODS_SELECTED.0 != 0,
            keyboard_cues: item_state & ODS_NOACCEL.0 == 0,
        }
    }
}

/// The text font, the glyph fonts and fallbacks for one DPI, deleted on drop. Loop thread only.
pub struct Fonts {
    dc: HDC,
    text: HFONT,
    marks: HFONT,
    icons: HFONT,
    em: i32,
    fallbacks: RefCell<[Option<Option<HFONT>>; FALLBACK_FACES.len()]>,
    windows_adds: i32,
}

impl Fonts {
    /// Segoe UI Variable Text at 14 epx, Segoe Fluent Icons at 12 epx (marks, chevron) and 16 epx (icons). Where a face
    /// is missing (a CI image may lack them; not checked), Segoe UI and Segoe MDL2 Assets stand in.
    pub fn new(dpi: u32) -> Result<Self, WinError> {
        // SAFETY: no input; the DC is deleted on drop.
        let dc = unsafe { CreateCompatibleDC(None) };
        if dc.is_invalid() {
            return Err(last_error("CreateCompatibleDC"));
        }
        let mut fonts = Self {
            dc,
            text: HFONT::default(),
            marks: HFONT::default(),
            icons: HFONT::default(),
            em: scale(FONT, dpi),
            fallbacks: RefCell::new([None; FALLBACK_FACES.len()]),
            windows_adds: 0,
        };
        // On an early return, drop frees what was made so far.
        fonts.text = fonts.font_with(&[TEXT_FACE, TEXT_FACE_OLD], scale(FONT, dpi))?;
        fonts.marks = fonts.font_with(&[GLYPH_FACE, GLYPH_FACE_OLD], scale(MARK, dpi))?;
        fonts.icons = fonts.font_with(&[GLYPH_FACE, GLYPH_FACE_OLD], scale(ICON, dpi))?;
        // A wrong guess only makes the menu wider than WinUI's, never cuts a text, so a failed read is no error.
        fonts.windows_adds = fonts.menu_font_adds(dpi).unwrap_or(0);
        Ok(fonts)
    }

    pub fn text_face(&self) -> String {
        self.face_of(self.text)
    }

    pub fn glyph_face(&self) -> String {
        self.face_of(self.icons)
    }

    /// What Windows adds to the width an owner-drawn item asks for: twice the average character width of the menu
    /// font (Wine's `menu.c`, from tests on Windows; Phase 1 saw 16 px at 120 dpi).
    pub fn windows_adds(&self) -> i32 {
        self.windows_adds
    }

    /// The text font, or the first fallback that has every character of `text` (the visible text, access key marks
    /// removed). When none has all, the one that misses the fewest; GDI's font linking may draw the rest.
    pub fn font_for(&self, text: &[u16]) -> HFONT {
        let check: Vec<u16> = text
            .iter()
            .copied()
            // A surrogate half never maps to a glyph through this call, and control characters draw nothing.
            .filter(|&u| u >= 0x20 && !(0xD800..0xE000).contains(&u))
            .collect();
        if check.is_empty() || self.missing(self.text, &check) == 0 {
            return self.text;
        }
        let mut best: Option<(usize, HFONT)> = None;
        for i in 0..FALLBACK_FACES.len() {
            let Some(font) = self.fallback(i) else {
                continue;
            };
            let missing = self.missing(font, &check);
            if missing == 0 {
                return font;
            }
            if best.is_none_or(|(fewest, _)| missing < fewest) {
                best = Some((missing, font));
            }
        }
        best.map_or(self.text, |(_, font)| font)
    }

    /// Made on first use and kept, a missing face too, so a label is checked against each face once per menu.
    fn fallback(&self, i: usize) -> Option<HFONT> {
        if let Some(slot) = self.fallbacks.borrow()[i] {
            return slot;
        }
        let font = create_font(FALLBACK_FACES[i], self.em)
            .ok()
            .and_then(|font| {
                if self.is_face(font, FALLBACK_FACES[i]) {
                    Some(font)
                } else {
                    // GDI's substitute for a missing face would be a second try of a face already tried.
                    delete(font.into());
                    None
                }
            });
        self.fallbacks.borrow_mut()[i] = Some(font);
        font
    }

    /// The first face of `faces` that is installed, at `px` em height. When none is, GDI's substitute for the last
    /// one: a menu with a plain font still works, where an error would leave no menu at all.
    fn font_with(&self, faces: &[&str], px: i32) -> Result<HFONT, WinError> {
        let mut last = None;
        for face in faces {
            if let Some(font) = last.take() {
                delete(HGDIOBJ::from(font));
            }
            let font = create_font(face, px)?;
            if self.is_face(font, face) {
                return Ok(font);
            }
            last = Some(font);
        }
        last.ok_or(WinError {
            call: "CreateFontIndirectW",
            code: ERROR_NOT_FOUND.0,
        })
    }

    fn is_face(&self, font: HFONT, face: &str) -> bool {
        self.face_of(font).eq_ignore_ascii_case(face)
    }

    fn face_of(&self, font: HFONT) -> String {
        let mut buf = [0u16; 64];
        // SAFETY: the DC and font live for the call; the old font is put back.
        let n = unsafe {
            let old = SelectObject(self.dc, font.into());
            let n = GetTextFaceW(self.dc, Some(&mut buf));
            SelectObject(self.dc, old);
            n
        };
        let len = usize::try_from(n).unwrap_or(0).min(buf.len());
        let end = buf[..len].iter().position(|&u| u == 0).unwrap_or(len);
        String::from_utf16_lossy(&buf[..end])
    }

    fn missing(&self, font: HFONT, text: &[u16]) -> usize {
        let mut glyphs = vec![0u16; text.len()];
        let Ok(count) = i32::try_from(text.len()) else {
            return text.len();
        };
        let wide: Vec<u16> = text.iter().copied().chain([0]).collect();
        // SAFETY: `wide` holds `count` units and a NUL; `glyphs` holds `count` slots; the old font is put back.
        let done = unsafe {
            let old = SelectObject(self.dc, font.into());
            let done = GetGlyphIndicesW(
                self.dc,
                PCWSTR(wide.as_ptr()),
                count,
                glyphs.as_mut_ptr(),
                GGI_MARK_NONEXISTING_GLYPHS,
            );
            SelectObject(self.dc, old);
            done
        };
        if done == u32::MAX {
            return text.len();
        }
        glyphs.iter().filter(|&&g| g == 0xFFFF).count()
    }

    /// Wine's `get_char_dimensions`: the average width of a to z and A to Z, rounded as Windows does.
    fn menu_font_adds(&self, dpi: u32) -> Result<i32, WinError> {
        let mut metrics = NONCLIENTMETRICSW {
            cbSize: size_of::<NONCLIENTMETRICSW>() as u32,
            ..Default::default()
        };
        // SAFETY: `metrics` is live with `cbSize` set, as the action requires.
        unsafe {
            SystemParametersInfoForDpi(
                SPI_GETNONCLIENTMETRICS.0,
                metrics.cbSize,
                Some((&raw mut metrics).cast::<c_void>()),
                0,
                dpi,
            )
        }
        .map_err(|e| win_error("SystemParametersInfoForDpi", &e))?;
        // SAFETY: a valid LOGFONTW; the font is deleted below.
        let font = unsafe { CreateFontIndirectW(&raw const metrics.lfMenuFont) };
        if font.is_invalid() {
            return Err(last_error("CreateFontIndirectW"));
        }
        let letters: Vec<u16> = ('a'..='z').chain('A'..='Z').map(|c| c as u16).collect();
        let mut size = SIZE::default();
        // SAFETY: the DC and font live for the call; the old font is put back.
        let ok = unsafe {
            let old = SelectObject(self.dc, font.into());
            let ok = GetTextExtentPoint32W(self.dc, &letters, &raw mut size);
            SelectObject(self.dc, old);
            ok
        };
        delete(font.into());
        if !ok.as_bool() {
            return Err(last_error("GetTextExtentPoint32W"));
        }
        Ok(2 * ((size.cx / 26 + 1) / 2))
    }

    fn text_width(&self, font: HFONT, text: &str, hide_prefix: bool) -> i32 {
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        let mut r = RECT::default();
        let prefix = if hide_prefix {
            DT_HIDEPREFIX
        } else {
            DRAW_TEXT_FORMAT(0)
        };
        // SAFETY: the DC and font live for the call; the old font is put back.
        unsafe {
            let old = SelectObject(self.dc, font.into());
            DrawTextW(
                self.dc,
                &mut wide,
                &raw mut r,
                DT_CALCRECT | DT_SINGLELINE | prefix,
            );
            SelectObject(self.dc, old);
        }
        r.right - r.left
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        for font in [self.text, self.marks, self.icons]
            .into_iter()
            .chain(self.fallbacks.borrow().iter().flatten().flatten().copied())
        {
            if !font.is_invalid() {
                delete(font.into());
            }
        }
        // SAFETY: the DC was made in `new` and no font is selected into it any more.
        let _ = unsafe { DeleteDC(self.dc) };
    }
}

fn create_font(face: &str, px: i32) -> Result<HFONT, WinError> {
    let mut lf = LOGFONTW {
        // Negative: the em height, as XAML's FontSize means it.
        lfHeight: -px,
        lfWeight: FW_NORMAL.0 as i32,
        lfCharSet: DEFAULT_CHARSET,
        // Follows the user's font smoothing setting, as Windows' own menus do.
        lfQuality: DEFAULT_QUALITY,
        ..Default::default()
    };
    for (slot, unit) in lf.lfFaceName.iter_mut().zip(face.encode_utf16()) {
        *slot = unit;
    }
    // SAFETY: `lf` is a valid LOGFONTW with a NUL-terminated face (the last slot stays 0 for names under 32 units).
    let font = unsafe { CreateFontIndirectW(&raw const lf) };
    if font.is_invalid() {
        return Err(last_error("CreateFontIndirectW"));
    }
    Ok(font)
}

fn delete(object: HGDIOBJ) {
    // SAFETY: the object was made by this module and is selected into no DC.
    let _ = unsafe { DeleteObject(object) };
}

/// The item's text as shown: `&&` gives `&`, a single `&` marks the access key and is dropped.
fn visible(text: &str) -> Vec<u16> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '&' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }
    out.encode_utf16().collect()
}

fn text_of(item: &MenuItem) -> Option<&str> {
    match item {
        MenuItem::Label(t)
        | MenuItem::Action { text: t, .. }
        | MenuItem::Submenu { text: t, .. } => Some(t),
        MenuItem::Separator => None,
    }
}

/// The access key is underlined only when Windows asks for keyboard cues; a label is a status line with no key.
fn prefix_format(item: &MenuItem, state: ItemState) -> DRAW_TEXT_FORMAT {
    if state.keyboard_cues && !matches!(item, MenuItem::Label(_)) {
        DRAW_TEXT_FORMAT(0)
    } else {
        DT_HIDEPREFIX
    }
}

/// What `WM_MEASUREITEM` asks for, `(itemWidth, itemHeight)`, less what Windows adds to the width.
pub fn measure(item: &MenuItem, fonts: &Fonts, layout: &Layout) -> (u32, u32) {
    let Some(t) = text_of(item) else {
        return (0, layout.separator.unsigned_abs());
    };
    let font = fonts.font_for(&visible(t));
    // A long status line ends in an ellipsis instead of stretching the menu across the screen; the screen reader
    // name keeps the whole text.
    let text_px = fonts
        .text_width(font, t, true)
        .min(scale(TEXT_MAX, layout.dpi));
    let wanted = layout.width_for(text_px, matches!(item, MenuItem::Submenu { .. }));
    let asked = (wanted - fonts.windows_adds()).max(1);
    (asked.unsigned_abs(), layout.row.unsigned_abs())
}

/// Paints one item into `dc` at `rect`, through an off-screen bitmap so nothing flickers. Returns the rect to take
/// out of the clip region for a submenu: Windows paints its own arrow after `WM_DRAWITEM`, and the Phase 1 spike
/// showed one chevron only once the whole item was excluded (`ExcludeClipRect`; a smaller clip was not tested).
pub fn paint_item(
    dc: HDC,
    rect: RECT,
    item: &MenuItem,
    state: ItemState,
    palette: &Palette,
    fonts: &Fonts,
    layout: &Layout,
) -> Result<Option<RECT>, WinError> {
    let clip = matches!(item, MenuItem::Submenu { .. }).then_some(rect);
    let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
    if w <= 0 || h <= 0 {
        return Ok(clip);
    }
    let mut canvas = Canvas::new(w, h)?;
    let enabled = match item {
        MenuItem::Action { enabled, .. } => *enabled,
        _ => true,
    };
    // WinUI never highlights a disabled item; a label is a status line, not a command.
    let hot = state.selected
        && enabled
        && matches!(item, MenuItem::Action { .. } | MenuItem::Submenu { .. });
    fill(canvas.pixels(), w, h, item, hot, layout, palette);
    if let Some(t) = text_of(item) {
        let text_colour = match item {
            // Information, not a command that is off: the secondary text reads at about 6:1 in light and 9:1 in
            // dark, where Phase 0's disabled grey read at 2.5 to 2.7:1.
            MenuItem::Label(_) => palette.secondary,
            _ if !enabled => palette.disabled,
            _ if hot => palette.hover_text,
            _ => palette.text,
        };
        let mark_colour = if !enabled {
            palette.disabled
        } else if hot {
            palette.hover_text
        } else {
            palette.secondary
        };
        let chevron_colour = if hot {
            palette.hover_secondary
        } else {
            palette.secondary
        };
        let line = |left: i32, right: i32| RECT {
            left,
            top: layout.line_top,
            right,
            bottom: layout.line_top + layout.line,
        };
        let dib = canvas.dc;
        // SAFETY: the canvas DC lives until `canvas` drops.
        unsafe { SetBkMode(dib, TRANSPARENT) };

        if let MenuItem::Action {
            command,
            checked: true,
            ..
        } = item
            && let Some(mark) = mark_for(*command)
        {
            let left = layout.mark_left;
            draw(
                dib,
                fonts.marks,
                &[mark.glyph()],
                line(left, left + layout.mark),
                mark_colour,
                DT_CENTER | DT_NOPREFIX,
            );
        }
        if layout.columns.icons
            && let Some(icon) = icon_for(item)
        {
            let left = layout.icon_left;
            draw(
                dib,
                fonts.icons,
                &[icon],
                line(left, left + layout.icon),
                text_colour,
                DT_CENTER | DT_NOPREFIX,
            );
        }
        let submenu = matches!(item, MenuItem::Submenu { .. });
        let right = w - if submenu {
            layout.chevron_right
        } else {
            layout.right
        };
        let wide: Vec<u16> = t.encode_utf16().collect();
        draw(
            dib,
            fonts.font_for(&visible(t)),
            &wide,
            line(layout.text_left, right.max(layout.text_left)),
            text_colour,
            DT_LEFT | DT_END_ELLIPSIS | prefix_format(item, state),
        );
        if submenu {
            let right = w - layout.right;
            draw(
                dib,
                fonts.marks,
                &[GLYPH_CHEVRON],
                line(right - layout.chevron, right),
                chevron_colour,
                DT_CENTER | DT_NOPREFIX,
            );
        }
    }
    // SAFETY: both DCs live for the call.
    unsafe {
        BitBlt(
            dc,
            rect.left,
            rect.top,
            w,
            h,
            Some(canvas.dc),
            0,
            0,
            SRCCOPY,
        )
    }
    .map_err(|e| win_error("BitBlt", &e))?;
    Ok(clip)
}

/// Background, hover shape and separator line, straight into the pixels, so the hover corners are smooth.
fn fill(
    px: &mut [u32],
    w: i32,
    h: i32,
    item: &MenuItem,
    hot: bool,
    layout: &Layout,
    palette: &Palette,
) {
    px.fill(pixel(palette.background));
    if matches!(item, MenuItem::Separator) {
        let top = layout.separator_top.min(h);
        let bottom = (top + layout.separator_line).min(h);
        for y in top..bottom {
            row_of(px, w, y).fill(pixel(palette.separator));
        }
        return;
    }
    if !hot {
        return;
    }
    let r = (
        layout.inset_x,
        layout.inset_y,
        w - layout.inset_x,
        h - layout.inset_y,
    );
    let radius = layout.radius as f32;
    for y in r.1.max(0)..r.3.min(h) {
        let line = row_of(px, w, y);
        for x in r.0.max(0)..r.2.min(w) {
            let a = coverage(x, y, r, radius);
            if a > 0.0 {
                line[x as usize] = pixel(lerp(palette.background, palette.hover, a));
            }
        }
    }
}

fn row_of(px: &mut [u32], w: i32, y: i32) -> &mut [u32] {
    let start = (y * w) as usize;
    &mut px[start..start + w as usize]
}

/// How much of pixel `(px, py)` a rounded rect covers: 1 inside, 0 outside, a fraction on the curve.
fn coverage(px: i32, py: i32, r: (i32, i32, i32, i32), radius: f32) -> f32 {
    let (x0, y0, x1, y1) = (r.0 as f32, r.1 as f32, r.2 as f32, r.3 as f32);
    let (cx, cy) = (px as f32 + 0.5, py as f32 + 0.5);
    if cx < x0 || cy < y0 || cx > x1 || cy > y1 {
        return 0.0;
    }
    let nx = cx.clamp(x0 + radius, x1 - radius);
    let ny = cy.clamp(y0 + radius, y1 - radius);
    let d = ((cx - nx).powi(2) + (cy - ny).powi(2)).sqrt();
    (radius - d + 0.5).clamp(0.0, 1.0)
}

fn draw(dc: HDC, font: HFONT, text: &[u16], mut r: RECT, colour: Rgb, format: DRAW_TEXT_FORMAT) {
    let mut buf = text.to_vec();
    // SAFETY: the DC and font live for the call; the old font is put back.
    unsafe {
        let old = SelectObject(dc, font.into());
        SetTextColor(dc, COLORREF(colour.colorref()));
        DrawTextW(
            dc,
            &mut buf,
            &raw mut r,
            format | DT_SINGLELINE | DT_VCENTER | DT_NOCLIP,
        );
        SelectObject(dc, old);
    }
}

/// A 32-bit top-down DIB selected into a memory DC.
struct Canvas {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u32,
    len: usize,
}

impl Canvas {
    fn new(w: i32, h: i32) -> Result<Self, WinError> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                // Negative: top-down rows, so row 0 is the top.
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        // SAFETY: no input.
        let dc = unsafe { CreateCompatibleDC(None) };
        if dc.is_invalid() {
            return Err(last_error("CreateCompatibleDC"));
        }
        let mut bits: *mut c_void = std::ptr::null_mut();
        // SAFETY: `info` describes a 32-bit DIB; `bits` receives its pixels, which live until the bitmap is deleted.
        let bitmap = match unsafe {
            CreateDIBSection(
                Some(dc),
                &raw const info,
                DIB_RGB_COLORS,
                &raw mut bits,
                None,
                0,
            )
        } {
            Ok(b) if !bits.is_null() => b,
            Ok(b) => {
                delete(b.into());
                // SAFETY: made above, nothing selected.
                let _ = unsafe { DeleteDC(dc) };
                return Err(WinError {
                    call: "CreateDIBSection",
                    code: ERROR_NOT_ENOUGH_MEMORY.0,
                });
            }
            Err(e) => {
                // SAFETY: made above, nothing selected.
                let _ = unsafe { DeleteDC(dc) };
                return Err(win_error("CreateDIBSection", &e));
            }
        };
        // SAFETY: both live; the old bitmap is put back on drop.
        let old = unsafe { SelectObject(dc, bitmap.into()) };
        Ok(Self {
            dc,
            bitmap,
            old,
            bits: bits.cast(),
            len: (w as usize) * (h as usize),
        })
    }

    fn pixels(&mut self) -> &mut [u32] {
        // GDI may still hold queued drawing for this bitmap, and the pixels must see it.
        // SAFETY: no input.
        let _ = unsafe { GdiFlush() };
        // SAFETY: `bits` points at `len` pixels owned by the bitmap, which lives as long as `self`.
        unsafe { std::slice::from_raw_parts_mut(self.bits, self.len) }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        // SAFETY: made in `new`; the old bitmap goes back before the DIB is deleted.
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

fn last_error(call: &'static str) -> WinError {
    // SAFETY: no input.
    WinError {
        call,
        code: unsafe { GetLastError() }.0,
    }
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
    use crate::tray::theme::Contrast;
    use wakeshot_core::holds::HoldKind;
    use wakeshot_core::text::Status;
    use wakeshot_core::view::{IconState, KeepAwake};
    use windows::Win32::Graphics::Gdi::{
        COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT,
        COLOR_HOTLIGHT, COLOR_WINDOW, COLOR_WINDOWTEXT, GetSysColor,
    };

    const DARK: Theme = Theme {
        light: false,
        contrast: None,
    };
    const LIGHT: Theme = Theme {
        light: true,
        contrast: None,
    };
    const BOTH: Columns = Columns {
        marks: true,
        icons: true,
    };

    fn action(command: MenuCommand, checked: bool, enabled: bool) -> MenuItem {
        MenuItem::Action {
            text: text::action(command),
            command,
            checked,
            enabled,
        }
    }

    fn keep_awake() -> Vec<MenuItem> {
        let mut items: Vec<MenuItem> = [
            KeepAwake::Off,
            KeepAwake::Minutes(30),
            KeepAwake::Minutes(60),
            KeepAwake::Minutes(120),
            KeepAwake::Minutes(240),
            KeepAwake::UntilOff,
        ]
        .into_iter()
        .map(|k| action(MenuCommand::KeepAwake(k), k == KeepAwake::Minutes(60), true))
        .collect();
        items.push(MenuItem::Separator);
        items.push(action(MenuCommand::KeepScreenOn(true), false, true));
        items
    }

    /// The product menu as `present` builds it: holding for two Claude Code sessions and Keep awake 1 hour.
    fn product_menu() -> Vec<MenuItem> {
        let status = Status {
            icon: IconState::Holding,
            stepped_back: false,
            requests_only: false,
            screen_on: false,
        };
        let groups = [
            text::hold_group(HoldKind::Agent, "Claude Code", 2),
            text::keep_awake_group(KeepAwake::Minutes(25)),
        ];
        vec![
            MenuItem::Label(text::status_line(&status, &groups)),
            MenuItem::Separator,
            action(MenuCommand::Automatic(false), true, true),
            MenuItem::Submenu {
                text: text::keep_awake_menu(),
                items: keep_awake(),
            },
            MenuItem::Separator,
            action(MenuCommand::Restore, false, true),
            action(MenuCommand::OpenLog, false, true),
            action(MenuCommand::Quit, false, true),
        ]
    }

    /// Each colour encodes its own index, so a test sees which system colour filled which field.
    fn sample_contrast() -> Contrast {
        let c = |i: u8| Rgb::new(i, i, 0x80);
        Contrast {
            window: c(5),
            window_text: c(8),
            highlight: c(13),
            highlight_text: c(14),
            gray_text: c(17),
            button_face: c(15),
            button_text: c(18),
            hot_light: c(26),
        }
    }

    /// What Phase 1's forced high contrast render used: this PC's system colours, whatever the mode.
    fn live_system_colours() -> Contrast {
        // SAFETY: documented indexes; the call only returns a value.
        let c = |i| Rgb::from_colorref(unsafe { GetSysColor(i) });
        Contrast {
            window: c(COLOR_WINDOW),
            window_text: c(COLOR_WINDOWTEXT),
            highlight: c(COLOR_HIGHLIGHT),
            highlight_text: c(COLOR_HIGHLIGHTTEXT),
            gray_text: c(COLOR_GRAYTEXT),
            button_face: c(COLOR_BTNFACE),
            button_text: c(COLOR_BTNTEXT),
            hot_light: c(COLOR_HOTLIGHT),
        }
    }

    /// One item painted at the origin of a canvas of its own size.
    struct Painted {
        px: Vec<u32>,
        w: i32,
        layout: Layout,
        palette: Palette,
        clip: Option<RECT>,
    }

    impl Painted {
        fn new(item: &MenuItem, state: ItemState, dpi: u32, theme: &Theme) -> Self {
            let fonts = Fonts::new(dpi).expect("fonts");
            let layout = Layout::new(dpi, BOTH);
            let palette = Palette::for_theme(theme);
            let w = scale(320, dpi);
            let h = if matches!(item, MenuItem::Separator) {
                layout.separator
            } else {
                layout.row
            };
            let mut canvas = Canvas::new(w, h).expect("canvas");
            let rect = RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            };
            let clip =
                paint_item(canvas.dc, rect, item, state, &palette, &fonts, &layout).expect("paint");
            Self {
                px: canvas.pixels().to_vec(),
                w,
                layout,
                palette,
                clip,
            }
        }

        fn at(&self, x: i32, y: i32) -> Rgb {
            let p = self.px[(y * self.w + x) as usize];
            Rgb::new((p >> 16) as u8, (p >> 8) as u8, p as u8)
        }

        /// Pixels in columns `x0..x1` of the text line that are not `fill`: ink of a text or a glyph.
        fn ink(&self, x0: i32, x1: i32, fill: Rgb) -> usize {
            let top = self.layout.line_top;
            (top..top + self.layout.line)
                .flat_map(|y| (x0..x1).map(move |x| (x, y)))
                .filter(|&(x, y)| self.at(x, y) != fill)
                .count()
        }
    }

    const MOUSE: ItemState = ItemState {
        selected: false,
        keyboard_cues: false,
    };
    const HOT: ItemState = ItemState {
        selected: true,
        keyboard_cues: false,
    };

    #[test]
    fn layout_matches_winui_at_96_dpi() {
        let l = Layout::new(96, Columns::default());
        assert_eq!(l.row, 33, "2 + 4 + 20 + 5 + 2");
        assert_eq!(l.separator, 3);
        assert_eq!((l.separator_top, l.separator_line), (1, 1));
        assert_eq!((l.inset_x, l.inset_y, l.radius), (4, 2, 4));
        assert_eq!((l.line_top, l.line), (6, 20));
        assert_eq!((l.mark, l.icon, l.chevron, l.font), (12, 16, 12, 14));
        assert_eq!(l.mark_left, 15, "item margin 4 + padding 11");
        assert_eq!(l.text_left, 15, "no columns");
        assert_eq!(l.right, 15);
        assert_eq!(
            l.chevron_right, 51,
            "gap 24 + chevron 12 + padding 11 + margin 4"
        );

        let marks = Layout::new(
            96,
            Columns {
                marks: true,
                icons: false,
            },
        );
        assert_eq!(marks.text_left, 43);
        let icons = Layout::new(
            96,
            Columns {
                marks: false,
                icons: true,
            },
        );
        assert_eq!((icons.icon_left, icons.text_left), (15, 43));
        let both = Layout::new(96, BOTH);
        assert_eq!(
            (both.mark_left, both.icon_left, both.text_left),
            (15, 43, 71)
        );
        assert_eq!(both.width_for(100, false), 71 + 100 + 15);
        assert_eq!(both.width_for(100, true), 71 + 100 + 51);
    }

    #[test]
    fn layout_scales_at_120_144_and_192_dpi() {
        // (dpi, row, separator, font, mark, icon, text_left with both columns, radius)
        for (dpi, row, separator, font, mark, icon, text_left, radius) in [
            (120, 41, 4, 18, 15, 20, 89, 5),
            (144, 50, 5, 21, 18, 24, 107, 6),
            (192, 66, 6, 28, 24, 32, 142, 8),
        ] {
            let l = Layout::new(dpi, BOTH);
            assert_eq!(l.row, row, "row at {dpi}");
            assert_eq!(l.separator, separator, "separator at {dpi}");
            assert_eq!(l.font, font, "font at {dpi}");
            assert_eq!(l.mark, mark, "mark at {dpi}");
            assert_eq!(l.icon, icon, "icon at {dpi}");
            assert_eq!(l.text_left, text_left, "text at {dpi}");
            assert_eq!(l.radius, radius, "radius at {dpi}");
            assert!(l.separator_line >= 1);
            assert!(
                l.line_top + l.line <= l.row - l.inset_y,
                "the line fits the row at {dpi}"
            );
            assert!(l.mark_left < l.icon_left && l.icon_left < l.text_left);
        }
    }

    #[test]
    fn palette_light_and_dark_match_the_tokens() {
        let light = Palette::for_theme(&LIGHT);
        assert_eq!(light.background, Rgb::new(0xf9, 0xf9, 0xf9));
        assert_eq!(light.text, Rgb::new(0x1a, 0x1a, 0x1a), "#E4000000");
        assert_eq!(light.secondary, Rgb::new(0x5f, 0x5f, 0x5f), "#9E000000");
        assert_eq!(light.disabled, Rgb::new(0x9f, 0x9f, 0x9f), "#5C000000");
        assert_eq!(light.hover, Rgb::new(0xf0, 0xf0, 0xf0), "#09000000");
        assert_eq!(light.separator, Rgb::new(0xea, 0xea, 0xea), "#0F000000");
        assert_eq!(light.border, Rgb::new(0xea, 0xea, 0xea), "#0F000000");

        let dark = Palette::for_theme(&DARK);
        assert_eq!(dark.background, Rgb::new(0x2c, 0x2c, 0x2c));
        assert_eq!(dark.text, Rgb::new(0xff, 0xff, 0xff));
        assert_eq!(dark.secondary, Rgb::new(0xcf, 0xcf, 0xcf), "#C5FFFFFF");
        assert_eq!(dark.disabled, Rgb::new(0x79, 0x79, 0x79), "#5DFFFFFF");
        assert_eq!(dark.hover, Rgb::new(0x38, 0x38, 0x38), "#0FFFFFFF");
        assert_eq!(dark.hover_text, Rgb::new(0xff, 0xff, 0xff));
        assert_eq!(dark.separator, Rgb::new(0x3d, 0x3d, 0x3d), "#15FFFFFF");
        assert_eq!(
            dark.border,
            Rgb::new(0x23, 0x23, 0x23),
            "#33000000, the Phase 1 border"
        );
    }

    #[test]
    fn palette_contrast_uses_system_colours() {
        let c = sample_contrast();
        for light in [false, true] {
            let p = Palette::for_theme(&Theme {
                light,
                contrast: Some(c),
            });
            assert_eq!(p.background, c.window);
            assert_eq!(p.text, c.button_text);
            assert_eq!(p.secondary, c.button_text);
            assert_eq!(p.disabled, c.gray_text);
            assert_eq!(p.hover, c.highlight);
            assert_eq!(p.hover_text, c.highlight_text);
            assert_eq!(p.hover_secondary, c.highlight_text);
            assert_eq!(p.separator, c.window_text);
            assert_eq!(p.border, c.window_text);
        }
    }

    #[test]
    fn marks_follow_the_command() {
        assert_eq!(mark_for(MenuCommand::Automatic(true)), Some(Mark::Check));
        assert_eq!(mark_for(MenuCommand::Automatic(false)), Some(Mark::Check));
        assert_eq!(mark_for(MenuCommand::KeepScreenOn(true)), Some(Mark::Check));
        for k in [KeepAwake::Off, KeepAwake::Minutes(30), KeepAwake::UntilOff] {
            assert_eq!(mark_for(MenuCommand::KeepAwake(k)), Some(Mark::Radio));
        }
        for c in [
            MenuCommand::Restore,
            MenuCommand::OpenLog,
            MenuCommand::Quit,
        ] {
            assert_eq!(mark_for(c), None);
        }
        assert_eq!(Mark::Check.glyph(), 0xE73E);
        assert_eq!(Mark::Radio.glyph(), 0xE915);
    }

    #[test]
    fn icons_only_for_commands_with_a_known_picture() {
        let menu = product_menu();
        let icons: Vec<Option<u16>> = menu.iter().map(icon_for).collect();
        assert_eq!(
            icons,
            [
                None,
                None,
                Some(ICON_AUTOMATIC),
                Some(ICON_KEEP_AWAKE),
                None,
                Some(ICON_RESTORE),
                Some(ICON_LOG),
                Some(ICON_QUIT),
            ]
        );
        assert!(keep_awake().iter().all(|i| icon_for(i).is_none()));
        let other = MenuItem::Submenu {
            text: "&Other".into(),
            items: Vec::new(),
        };
        assert_eq!(icon_for(&other), None, "a new submenu needs its own pick");
        // Haik's pick for Keep awake (2026-11-05): the clock "Recent".
        assert_eq!(ICON_KEEP_AWAKE, 0xE823);
    }

    #[test]
    fn item_glyphs_are_in_both_icon_fonts() {
        // Where Segoe Fluent Icons is missing the menu draws with Segoe MDL2 Assets, and a glyph a font lacks draws as
        // an empty box, so every glyph the menu uses must be in both.
        const ALL: [u16; 8] = [
            GLYPH_CHECK,
            GLYPH_RADIO,
            GLYPH_CHEVRON,
            ICON_AUTOMATIC,
            ICON_KEEP_AWAKE,
            ICON_RESTORE,
            ICON_LOG,
            ICON_QUIT,
        ];
        let fonts = Fonts::new(96).expect("fonts");
        for face in [GLYPH_FACE, GLYPH_FACE_OLD] {
            let font = create_font(face, scale(ICON, 96)).expect("font");
            let installed = fonts.is_face(font, face);
            let missing: Vec<String> = ALL
                .iter()
                .filter(|&&g| fonts.missing(font, &[g]) > 0)
                .map(|g| format!("{g:04X}"))
                .collect();
            // "Clock" E917 is not in MDL2 (its cmap, `docs/research/brand-windows.md`); seeing that shows the check
            // can find a missing glyph at all.
            let control = fonts.missing(font, &[0xE917]);
            delete(font.into());
            if !installed {
                println!("{face} is not installed here: not checked");
                continue;
            }
            assert!(missing.is_empty(), "{face} lacks {missing:?}");
            if face == GLYPH_FACE_OLD {
                assert_eq!(control, 1, "{face} has E917 now, or the check sees no gap");
            }
        }
    }

    #[test]
    fn columns_follow_marks_and_icons() {
        assert_eq!(Columns::of(&product_menu()), BOTH);
        assert_eq!(
            Columns::of(&keep_awake()),
            Columns {
                marks: true,
                icons: false
            }
        );
        let plain = [action(MenuCommand::Quit, false, true)];
        assert_eq!(
            Columns::of(&plain),
            Columns {
                marks: false,
                icons: true
            }
        );
        assert_eq!(Columns::of(&[]), Columns::default());
    }

    #[test]
    fn access_key_underline_only_with_keyboard_cues() {
        assert_eq!(
            ItemState::from_ods(0),
            ItemState {
                selected: false,
                keyboard_cues: true
            }
        );
        assert_eq!(
            ItemState::from_ods(ODS_NOACCEL.0 | ODS_SELECTED.0),
            ItemState {
                selected: true,
                keyboard_cues: false
            }
        );
        let quit = action(MenuCommand::Quit, false, true);
        let cues = ItemState {
            selected: false,
            keyboard_cues: true,
        };
        assert_eq!(
            prefix_format(&quit, cues),
            DRAW_TEXT_FORMAT(0),
            "underlined"
        );
        assert_eq!(prefix_format(&quit, MOUSE), DT_HIDEPREFIX);
        let label = MenuItem::Label("Holding awake".into());
        assert_eq!(
            prefix_format(&label, cues),
            DT_HIDEPREFIX,
            "a status line has no key"
        );

        // Painted: with cues the underline adds ink below the text; without, it does not.
        let with = Painted::new(&quit, cues, 96, &DARK);
        let without = Painted::new(&quit, MOUSE, 96, &DARK);
        let l = with.layout;
        let ink = |p: &Painted| p.ink(l.text_left, l.text_left + 60, p.palette.background);
        assert!(
            ink(&with) > ink(&without),
            "{} > {}",
            ink(&with),
            ink(&without)
        );
    }

    #[test]
    fn visible_text_drops_access_key_marks() {
        let v = |s: &str| String::from_utf16(&visible(s)).expect("utf-16");
        assert_eq!(v("&Keep awake"), "Keep awake");
        assert_eq!(v("Keep &screen on"), "Keep screen on");
        assert_eq!(v("Save && &Quit"), "Save & Quit");
        assert_eq!(v("trailing &"), "trailing ");
    }

    #[test]
    fn label_never_highlights() {
        let label = MenuItem::Label("Holding awake: Claude Code (2 sessions)".into());
        let p = Painted::new(&label, HOT, 96, &DARK);
        let (x, y) = (p.layout.inset_x + 2, p.layout.row / 2);
        assert_eq!(p.at(x, y), p.palette.background, "no hover fill on a label");
        assert!(
            p.ink(
                p.layout.text_left,
                p.layout.text_left + 40,
                p.palette.background
            ) > 0
        );

        let disabled = action(MenuCommand::KeepScreenOn(true), false, false);
        let p = Painted::new(&disabled, HOT, 96, &DARK);
        assert_eq!(
            p.at(x, y),
            p.palette.background,
            "no hover fill on a disabled item"
        );
    }

    #[test]
    fn paints_offscreen() {
        for dpi in [96, 144] {
            for theme in [DARK, LIGHT] {
                let palette = Palette::for_theme(&theme);
                let l = Layout::new(dpi, BOTH);
                let hover_spot = (l.inset_x + 2, l.row / 2);

                // A checked toggle, hot: hover fill, the mark, the icon and the text.
                let auto = action(MenuCommand::Automatic(false), true, true);
                let p = Painted::new(&auto, HOT, dpi, &theme);
                assert_eq!(
                    p.at(hover_spot.0, hover_spot.1),
                    palette.hover,
                    "hover at {dpi}"
                );
                assert_eq!(
                    p.at(0, hover_spot.1),
                    palette.background,
                    "the inset keeps the fill"
                );
                assert_eq!(
                    p.at(l.inset_x, l.inset_y),
                    palette.background,
                    "a round corner"
                );
                assert!(
                    p.ink(l.mark_left, l.mark_left + l.mark, palette.hover) > 0,
                    "check at {dpi}"
                );
                assert!(
                    p.ink(l.icon_left, l.icon_left + l.icon, palette.hover) > 0,
                    "icon at {dpi}"
                );
                assert!(
                    p.ink(l.text_left, l.text_left + l.font * 3, palette.hover) > 0,
                    "text at {dpi}"
                );
                assert_eq!(p.clip, None);

                // Unchecked and at rest: no fill and no mark, but the icon stays.
                let off = action(MenuCommand::Automatic(true), false, true);
                let p = Painted::new(&off, MOUSE, dpi, &theme);
                assert_eq!(p.at(hover_spot.0, hover_spot.1), palette.background);
                assert_eq!(
                    p.ink(l.mark_left, l.mark_left + l.mark, palette.background),
                    0,
                    "no mark"
                );
                assert!(p.ink(l.icon_left, l.icon_left + l.icon, palette.background) > 0);

                // A chosen length: the bullet.
                let hour = action(MenuCommand::KeepAwake(KeepAwake::Minutes(60)), true, true);
                let p = Painted::new(&hour, MOUSE, dpi, &theme);
                assert!(
                    p.ink(l.mark_left, l.mark_left + l.mark, palette.background) > 0,
                    "bullet"
                );

                // The submenu: chevron at the right, and the whole item to clip.
                let sub = MenuItem::Submenu {
                    text: text::keep_awake_menu(),
                    items: keep_awake(),
                };
                let p = Painted::new(&sub, MOUSE, dpi, &theme);
                let right = p.w - l.right;
                assert!(
                    p.ink(right - l.chevron, right, palette.background) > 0,
                    "chevron at {dpi}"
                );
                assert_eq!(
                    p.clip,
                    Some(RECT {
                        left: 0,
                        top: 0,
                        right: p.w,
                        bottom: l.row
                    })
                );

                // The separator: one line in the separator colour, the fill around it.
                let p = Painted::new(&MenuItem::Separator, HOT, dpi, &theme);
                for x in [0, p.w / 2, p.w - 1] {
                    assert_eq!(p.at(x, l.separator_top), palette.separator, "line at {dpi}");
                    assert_eq!(p.at(x, 0), palette.background, "above the line at {dpi}");
                    assert_eq!(
                        p.at(x, l.separator - 1),
                        palette.background,
                        "below at {dpi}"
                    );
                }
            }
        }
    }

    #[test]
    fn paints_at_the_rect_given() {
        let fonts = Fonts::new(96).expect("fonts");
        let layout = Layout::new(96, BOTH);
        let palette = Palette::for_theme(&DARK);
        let (w, h) = (300, 60);
        let mut canvas = Canvas::new(w, h).expect("canvas");
        canvas.pixels().fill(0x00ff_00ff);
        let rect = RECT {
            left: 3,
            top: 5,
            right: 203,
            bottom: 5 + layout.row,
        };
        let quit = action(MenuCommand::Quit, false, true);
        paint_item(canvas.dc, rect, &quit, MOUSE, &palette, &fonts, &layout).expect("paint");
        let px = canvas.pixels();
        let at = |x: i32, y: i32| px[(y * w + x) as usize];
        assert_eq!(at(2, 10), 0x00ff_00ff, "left of the rect is untouched");
        assert_eq!(at(203, 10), 0x00ff_00ff, "right of the rect is untouched");
        assert_eq!(at(10, 4), 0x00ff_00ff, "above the rect is untouched");
        assert_eq!(at(3, 5), pixel(palette.background));
        assert_eq!(at(202, 5 + layout.row - 1), pixel(palette.background));
    }

    #[test]
    fn non_latin_labels_get_a_font_that_has_them() {
        let fonts = Fonts::new(96).expect("fonts");
        // "Holding awake" in Armenian and in Russian, as escapes so the source stays ASCII.
        let armenian = "\u{54a}\u{561}\u{570}\u{578}\u{582}\u{574} \u{567} \u{561}\u{580}\u{569}\u{578}\u{582}\u{576}";
        let russian = "\u{41d}\u{435} \u{434}\u{430}\u{451}\u{442} \u{443}\u{441}\u{43d}\u{443}\u{442}\u{44c}";
        for label in [armenian, russian] {
            let shown = visible(label);
            let font = fonts.font_for(&shown);
            println!(
                "{label:?}: text font {:?} misses {}; drawn with {:?}",
                fonts.text_face(),
                fonts.missing(fonts.text, &shown),
                fonts.face_of(font)
            );
            assert_eq!(
                fonts.missing(font, &shown),
                0,
                "{label:?} drawn with {:?}",
                fonts.face_of(font)
            );
            if fonts.missing(fonts.text, &shown) > 0 {
                assert_ne!(font, fonts.text, "a font that has them, not the text font");
            }
        }
        // Latin stays on the text font.
        assert_eq!(fonts.font_for(&visible("&Quit")), fonts.text);
        // Measure and paint take the same font, so an Armenian label is as wide as its glyphs.
        let layout = Layout::new(96, BOTH);
        let label = MenuItem::Label(armenian.into());
        let (w, _) = measure(&label, &fonts, &layout);
        assert!(w > 60, "measured {w}");
    }

    #[test]
    fn measure_asks_less_what_windows_adds() {
        for dpi in [96, 120, 144] {
            let fonts = Fonts::new(dpi).expect("fonts");
            let layout = Layout::new(dpi, BOTH);
            let quit = action(MenuCommand::Quit, false, true);
            let text_px = fonts.text_width(fonts.text, "&Quit", true);
            let (w, h) = measure(&quit, &fonts, &layout);
            println!(
                "dpi {dpi}: Windows adds {} px, faces {:?} and {:?}",
                fonts.windows_adds(),
                fonts.text_face(),
                fonts.glyph_face()
            );
            assert_eq!(
                i64::from(w),
                i64::from(layout.width_for(text_px, false) - fonts.windows_adds())
            );
            assert_eq!(i64::from(h), i64::from(layout.row));
            assert_eq!(
                measure(&MenuItem::Separator, &fonts, &layout),
                (0, layout.separator.unsigned_abs())
            );
        }
    }

    #[test]
    fn a_long_label_is_capped() {
        let fonts = Fonts::new(96).expect("fonts");
        let layout = Layout::new(96, BOTH);
        let long = MenuItem::Label("Holding awake: ".repeat(20));
        let (w, _) = measure(&long, &fonts, &layout);
        let cap = layout.width_for(scale(TEXT_MAX, 96), false) - fonts.windows_adds();
        assert_eq!(i64::from(w), i64::from(cap));
    }

    /// Writes the review renders as BMP into `WAKESHOT_RENDER_DIR` (a PNG writer is `p5-icon-art`'s): the product
    /// menu and its Keep awake submenu side by side, at 96, 120 and 144 dpi, light, dark and high contrast (this PC's
    /// system colours, as Phase 1's forced render did). Run: `cargo test -p wakeshot-windows renders_for_review --
    /// --ignored`.
    #[test]
    #[ignore = "writes pictures for review"]
    fn renders_for_review() {
        let Some(dir) = std::env::var_os("WAKESHOT_RENDER_DIR") else {
            println!("set WAKESHOT_RENDER_DIR");
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        let main = product_menu();
        let sub = keep_awake();
        let contrast = Theme {
            light: false,
            contrast: Some(live_system_colours()),
        };
        for dpi in [96u32, 120, 144] {
            for (name, theme) in [("light", LIGHT), ("dark", DARK), ("contrast", contrast)] {
                // The main menu with Keep awake open, and in the submenu "2 hours" under the mouse.
                let file = dir.join(format!("menu-render-{dpi}-{name}.bmp"));
                render(
                    &[(&main, Some(3)), (&sub, Some(3))],
                    dpi,
                    &theme,
                    MOUSE.keyboard_cues,
                    &file,
                );
                println!("wrote {}", file.display());
            }
            // Scratch check: an Armenian status line and keyboard cues.
            let armenian = vec![
                MenuItem::Label(
                    "\u{54a}\u{561}\u{570}\u{578}\u{582}\u{574} \u{567} \u{561}\u{580}\u{569}\u{578}\u{582}\u{576}: Claude Code (2 sessions)".into(),
                ),
                MenuItem::Separator,
                action(MenuCommand::Automatic(false), true, true),
                action(MenuCommand::Quit, false, true),
            ];
            let file = dir.join(format!("menu-check-armenian-{dpi}.bmp"));
            render(&[(&armenian, Some(2))], dpi, &DARK, true, &file);
        }
    }

    fn render(
        menus: &[(&Vec<MenuItem>, Option<usize>)],
        dpi: u32,
        theme: &Theme,
        cues: bool,
        file: &std::path::Path,
    ) {
        let fonts = Fonts::new(dpi).expect("fonts");
        let palette = Palette::for_theme(theme);
        let gap = scale(8, dpi);
        let sizes: Vec<(Layout, i32, i32)> = menus
            .iter()
            .map(|(items, _)| {
                let layout = Layout::new(dpi, Columns::of(items));
                // What Windows draws: the width asked plus what it adds.
                let width = items
                    .iter()
                    .map(|i| measure(i, &fonts, &layout).0 as i32 + fonts.windows_adds())
                    .max()
                    .unwrap_or(0);
                let height = items
                    .iter()
                    .map(|i| measure(i, &fonts, &layout).1 as i32)
                    .sum();
                (layout, width, height)
            })
            .collect();
        let w: i32 = sizes.iter().map(|s| s.1).sum::<i32>() + gap * (menus.len() as i32 - 1);
        let h = sizes.iter().map(|s| s.2).max().unwrap_or(0);
        let mut canvas = Canvas::new(w, h).expect("canvas");
        canvas.pixels().fill(0x0080_8080);
        let mut x = 0;
        for ((items, hot), (layout, width, _)) in menus.iter().zip(&sizes) {
            let mut y = 0;
            for (i, item) in items.iter().enumerate() {
                let height = measure(item, &fonts, layout).1 as i32;
                let rect = RECT {
                    left: x,
                    top: y,
                    right: x + width,
                    bottom: y + height,
                };
                let state = ItemState {
                    selected: *hot == Some(i),
                    keyboard_cues: cues,
                };
                paint_item(canvas.dc, rect, item, state, &palette, &fonts, layout).expect("paint");
                y += height;
            }
            x += width + gap;
        }
        let px = canvas.pixels().to_vec();
        std::fs::write(file, bmp(w, h, &px)).expect("write");
        println!(
            "{}: {w} x {h}, row {} px, font {} px, glyphs {} px, faces {:?} {:?}",
            file.display(),
            sizes[0].0.row,
            sizes[0].0.font,
            sizes[0].0.mark,
            fonts.text_face(),
            fonts.glyph_face()
        );
    }

    /// A 32-bit top-down BMP.
    fn bmp(w: i32, h: i32, px: &[u32]) -> Vec<u8> {
        let data = px.len() * 4;
        let mut out = Vec::with_capacity(54 + data);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&((54 + data) as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&(-h).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&[0; 24]);
        for p in px {
            out.extend_from_slice(&(p | 0xff00_0000).to_le_bytes());
        }
        out
    }
}
