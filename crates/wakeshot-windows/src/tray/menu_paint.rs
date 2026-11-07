//! Menu layout, colours and item painting, with no window.
//!
//! The drawing half of the owner-drawn menu (`docs/research/tray-menu-look.md`, "Pick"): `menu` calls [`measure`] from
//! `WM_MEASUREITEM` and [`paint_item`] from `WM_DRAWITEM`. Sizes and colours are the WinUI 3 `MenuFlyout` template's
//! (microsoft-ui-xaml `MenuFlyout_themeresources.xaml`), so the menu matches the XAML menus of Explorer and the
//! taskbar, while Windows keeps the menu loop, keyboard and screen readers.

use std::cell::RefCell;
use std::ffi::c_void;

use wakeshot_core::text;
use wakeshot_core::view::{IconState, MenuCommand, MenuItem};
use windows::Win32::Foundation::{
    COLORREF, ERROR_NOT_ENOUGH_MEMORY, ERROR_NOT_FOUND, GetLastError, RECT, SIZE, WIN32_ERROR,
};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CLEARTYPE_QUALITY, CreateCompatibleDC,
    CreateDIBSection, CreateFontIndirectW, DEFAULT_CHARSET, DIB_RGB_COLORS, DRAW_TEXT_FORMAT,
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
use crate::tray::art::{self, GlyphColours};
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
/// The widest text an item asks room for: two hold groups fit, more end in an ellipsis. WinUI sets no limit (its
/// `MenuFlyoutPresenter` has a minimum width only and stops at the monitor's edge; 456 epx is the plain `Flyout`'s);
/// ours keeps a long hold reason from stretching the menu across the screen.
const TEXT_MAX: i32 = 480;

const GLYPH_CHECK: u16 = 0xE73E;
const GLYPH_RADIO: u16 = 0xE915;
const GLYPH_CHEVRON: u16 = 0xE974;

// Item icons, Segoe Fluent Icons (the same code points are in Segoe MDL2 Assets): an eye for Automatic, which watches
// agents; a clock for Keep awake, since each of its choices is a length of time ("Recent" E823, because "Clock" E917
// draws the same clock but is missing from MDL2); the restore arrow; a page for the log; the close cross for Quit,
// since a power button would read as shutting the PC down.
const ICON_AUTOMATIC: u16 = 0xE890;
const ICON_KEEP_AWAKE: u16 = 0xE823;
const ICON_RESTORE: u16 = 0xE777;
const ICON_LOG: u16 = 0xE8A5;
const ICON_QUIT: u16 = 0xE711;

const TEXT_FACE: &str = "Segoe UI Variable Text";
const TEXT_FACE_OLD: &str = "Segoe UI";
const GLYPH_FACE: &str = "Segoe Fluent Icons";
const GLYPH_FACE_OLD: &str = "Segoe MDL2 Assets";
/// Faces for a label the text font cannot draw (hold reasons are user text, and Segoe UI Variable lacks Armenian, one
/// of the languages Wakeshot ships in). Segoe UI first, then the Windows UI font of each script group.
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

/// `epx` at `dpi`, rounded to the nearest pixel as `MulDiv` rounds, so 33 epx is 41 px at 120 dpi.
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
    /// Marks at rest and the chevron.
    pub secondary: Rgb,
    /// The status line: WinUI's secondary text. The same colour as `secondary` except in high contrast, where WinUI
    /// gives marks and the chevron the commands' `ButtonText` and secondary text `WindowText`.
    pub label: Rgb,
    pub disabled: Rgb,
    pub hover: Rgb,
    pub hover_text: Rgb,
    pub hover_secondary: Rgb,
    pub separator: Rgb,
    /// For `DWMWA_BORDER_COLOR` on the menu window.
    pub border: Rgb,
    /// The tray glyph on the status line: the tray icon's own colours for this mode, so the light is Moss while
    /// holding; in high contrast one colour, the status line's text colour, as every other glyph of a row takes.
    pub status: GlyphColours,
}

impl Palette {
    pub fn for_theme(theme: &Theme) -> Self {
        if let Some(c) = theme.contrast {
            return Self {
                background: c.window,
                text: c.button_text,
                secondary: c.button_text,
                // `TextFillColorSecondaryBrush` and the menu's own secondary text
                // (`MenuFlyoutItemKeyboardAcceleratorTextForeground`) are `WindowText` in WinUI's high contrast
                // dictionary, the colour Microsoft gives UI that cannot be picked, so the status line does not look
                // like one more command.
                label: c.window_text,
                disabled: c.gray_text,
                hover: c.highlight,
                hover_text: c.highlight_text,
                hover_secondary: c.highlight_text,
                separator: c.window_text,
                border: c.window_text,
                status: GlyphColours::one(c.window_text.into()),
            };
        }
        // ARGB tokens from `Common_themeresources_any.xaml`. The fills are the classic menu's own, measured on screen.
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
            label: over(secondary, background),
            disabled: over(disabled, background),
            hover,
            hover_text: over(primary, hover),
            hover_secondary: over(secondary, hover),
            separator: over(divider, background),
            border: over(stroke, background),
            // The tray's own pair for a taskbar of this mode; both keep 3:1 on this fill too (test
            // `status_glyph_keeps_3_to_1`).
            status: if theme.light {
                art::ON_LIGHT
            } else {
                art::ON_DARK
            },
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

/// What an item shows in the icon column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// A glyph of the icon font, in the item's text colour.
    Font(u16),
    /// The tray icon's glyph for this state (`art::tray_mask`), in `Palette::status`.
    Tray(IconState),
}

/// The icon of an item. Only items with a picture Windows users know get one (Microsoft's menu guidance:
/// "Don't feel obligated to provide icons for commands that don't have a standard visualization"): the Keep awake
/// lengths and Keep screen on are a list of choices with marks. The status line shows the tray's own glyph for the
/// state it names, so the column WinUI keeps empty beside it says the state as the tray does.
pub fn icon_for(item: &MenuItem) -> Option<Icon> {
    match item {
        MenuItem::Action { command, .. } => match command {
            MenuCommand::Automatic(_) => Some(Icon::Font(ICON_AUTOMATIC)),
            MenuCommand::Restore => Some(Icon::Font(ICON_RESTORE)),
            MenuCommand::OpenLog => Some(Icon::Font(ICON_LOG)),
            MenuCommand::Quit => Some(Icon::Font(ICON_QUIT)),
            MenuCommand::KeepAwake(_) | MenuCommand::KeepScreenOn(_) => None,
        },
        MenuItem::Submenu { text: t, .. } if *t == text::keep_awake_menu() => {
            Some(Icon::Font(ICON_KEEP_AWAKE))
        }
        MenuItem::Label { icon, .. } => icon.map(Icon::Tray),
        MenuItem::Submenu { .. } | MenuItem::Separator => None,
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
    /// font (Wine's `menu.c`, from tests on Windows; 16 px at 120 dpi when measured).
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
        // ClearType whatever the user's smoothing setting, so `draw` gets three coverages per pixel to fold into one
        // grey and the text weighs the same on every PC; where GDI cannot ("when possible", says LOGFONT), the text
        // comes grey or plain and the fold leaves it so. GDI's own grayscale (`ANTIALIASED_QUALITY`) drew text about a
        // fifth lighter than Explorer's (`docs/research.md`, "Menu text in grey").
        lfQuality: CLEARTYPE_QUALITY,
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
        MenuItem::Label { text: t, .. }
        | MenuItem::Action { text: t, .. }
        | MenuItem::Submenu { text: t, .. } => Some(t),
        MenuItem::Separator => None,
    }
}

/// The access key is underlined only when Windows asks for keyboard cues; a label is a status line with no key.
fn prefix_format(item: &MenuItem, state: ItemState) -> DRAW_TEXT_FORMAT {
    if state.keyboard_cues && !matches!(item, MenuItem::Label { .. }) {
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
/// out of the clip region for a submenu: Windows paints its own arrow after `WM_DRAWITEM`, and a test showed one
/// chevron only once the whole item was excluded (`ExcludeClipRect`; a smaller clip was not tested).
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
            // dark, where the classic menu's disabled grey read at 2.5 to 2.7:1.
            MenuItem::Label { .. } => palette.label,
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
        // SAFETY: the canvas DC lives until `canvas` drops.
        unsafe { SetBkMode(canvas.dc, TRANSPARENT) };

        if let MenuItem::Action {
            command,
            checked: true,
            ..
        } = item
            && let Some(mark) = mark_for(*command)
        {
            let left = layout.mark_left;
            draw(
                &mut canvas,
                fonts.marks,
                &[mark.glyph()],
                line(left, left + layout.mark),
                mark_colour,
                DT_CENTER | DT_NOPREFIX,
            );
        }
        if layout.columns.icons {
            let left = layout.icon_left;
            match icon_for(item) {
                Some(Icon::Font(glyph)) => draw(
                    &mut canvas,
                    fonts.icons,
                    &[glyph],
                    line(left, left + layout.icon),
                    text_colour,
                    DT_CENTER | DT_NOPREFIX,
                ),
                Some(Icon::Tray(icon)) => {
                    tray_glyph(&mut canvas, w, h, icon, layout, palette.status)
                }
                None => {}
            }
        }
        let submenu = matches!(item, MenuItem::Submenu { .. });
        let right = w - if submenu {
            layout.chevron_right
        } else {
            layout.right
        };
        let wide: Vec<u16> = t.encode_utf16().collect();
        draw(
            &mut canvas,
            fonts.font_for(&visible(t)),
            &wide,
            line(layout.text_left, right.max(layout.text_left)),
            text_colour,
            DT_LEFT | DT_END_ELLIPSIS | prefix_format(item, state),
        );
        if submenu {
            let right = w - layout.right;
            draw(
                &mut canvas,
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

/// Draws `text` in `colour`, then turns the ClearType pixels it made grey, as Windows' own XAML menus draw: no colour
/// fringes, which turned the grey check red-brown.
fn draw(
    canvas: &mut Canvas,
    font: HFONT,
    text: &[u16],
    mut r: RECT,
    colour: Rgb,
    format: DRAW_TEXT_FORMAT,
) {
    let under = canvas.pixels().to_vec();
    let mut buf = text.to_vec();
    // SAFETY: the DC and font live for the call; the old font is put back.
    unsafe {
        let old = SelectObject(canvas.dc, font.into());
        SetTextColor(canvas.dc, COLORREF(colour.to_colorref()));
        DrawTextW(
            canvas.dc,
            &mut buf,
            &raw mut r,
            format | DT_SINGLELINE | DT_VCENTER | DT_NOCLIP,
        );
        SelectObject(canvas.dc, old);
    }
    // Only the pixels this text changed, each against what was under it, so a neighbour's ink is never refolded.
    for (p, &u) in canvas.pixels().iter_mut().zip(&under) {
        if (*p ^ u) & 0x00ff_ffff != 0 {
            *p = pixel(fold(rgb_of(*p), rgb_of(u), colour));
        }
    }
}

fn rgb_of(p: u32) -> Rgb {
    Rgb::new((p >> 16) as u8, (p >> 8) as u8, p as u8)
}

/// The tray icon's glyph for `icon`, drawn at the icon size by the tray's own art (`art::tray_mask`, so the hand-made
/// frames at 16, 20 and 24 px come with it) in the box the font icons of the rows below take: the column's left edge,
/// centred on the text line. Laid over the row by its alpha, as the shell lays the tray icon over the taskbar.
fn tray_glyph(
    canvas: &mut Canvas,
    w: i32,
    h: i32,
    icon: IconState,
    layout: &Layout,
    colours: GlyphColours,
) {
    let size = layout.icon;
    let Ok(side) = u32::try_from(size) else {
        return;
    };
    if side == 0 {
        return;
    }
    let glyph = art::colorize(&art::tray_mask(icon, side), colours);
    let (left, top) = glyph_origin(layout);
    let px = canvas.pixels();
    for (i, &[r, g, b, alpha]) in glyph.pixels.as_chunks::<4>().0.iter().enumerate() {
        let i = i as i32;
        let (x, y) = (left + i % size, top + i / size);
        if alpha == 0 || !(0..w).contains(&x) || !(0..h).contains(&y) {
            continue;
        }
        let at = (y * w + x) as usize;
        let mixed = lerp(rgb_of(px[at]), Rgb::new(r, g, b), f32::from(alpha) / 255.0);
        px[at] = pixel(mixed);
    }
}

/// The top left corner of the status glyph's box in the item.
fn glyph_origin(layout: &Layout) -> (i32, i32) {
    (
        layout.icon_left,
        layout.line_top + (layout.line - layout.icon) / 2,
    )
}

/// One grey for a ClearType pixel. GDI moved each channel of `under` toward `ink` by the coverage of its own subpixel;
/// one blend by their mean keeps ClearType's weight and shape, since Explorer's grayscale text is as heavy (research
/// "Menu text in grey"). Each channel counts by how far `ink` is from `under` in it: a channel where the two are
/// equal says nothing, and in light and dark, all greys, every channel counts the same.
fn fold(drawn: Rgb, under: Rgb, ink: Rgb) -> Rgb {
    let (mut moved, mut span) = (0i32, 0i32);
    for (d, u, i) in [
        (drawn.r, under.r, ink.r),
        (drawn.g, under.g, ink.g),
        (drawn.b, under.b, ink.b),
    ] {
        let (d, u, i) = (i32::from(d), i32::from(u), i32::from(i));
        moved += (d - u) * (i - u).signum();
        span += (i - u).abs();
    }
    if span == 0 {
        return drawn;
    }
    lerp(under, ink, (moved as f32 / span as f32).clamp(0.0, 1.0))
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
mod tests;
