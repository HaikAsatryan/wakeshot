// Owner-drawn tray menu in the Windows 11 look (card `p1-tray-menu-look`). Spike code.
//
// Sizes are effective pixels (epx, 96 dpi) from the WinUI 3 MenuFlyout template
// (microsoft-ui-xaml `MenuFlyout_themeresources.xaml`, read 2026-11-03), so the menu matches the XAML menus
// Explorer and the taskbar use, while Windows keeps the menu loop, keyboard, placement and accessibility.

use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    path::Path,
};

use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::{
            Dwm::{
                DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
                DwmSetWindowAttribute,
            },
            Gdi::*,
        },
        UI::{
            Accessibility::{MSAA_MENU_SIG, MSAAMENUINFO},
            Controls::{
                DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_CHECKED, ODS_DISABLED, ODS_GRAYED,
                ODS_NOACCEL, ODS_SELECTED,
            },
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            WindowsAndMessaging::*,
        },
    },
    core::{PCWSTR, PWSTR, w},
};

use crate::log;

const MARGIN_X: i32 = 4;
const MARGIN_Y: i32 = 2;
const PAD_X: i32 = 11;
// The "narrow" padding WinUI uses when the menu was opened with mouse, pen or keyboard.
const PAD_TOP: i32 = 4;
const PAD_BOTTOM: i32 = 5;
const LINE: i32 = 20;
const CHECK_COL: i32 = 28;
const GLYPH: i32 = 12;
const CHEVRON_GAP: i32 = 24;
const SEP_PAD: i32 = 1;
const SEP_LINE: i32 = 1;
const RADIUS: i32 = 4;
const FONT_PX: i32 = 14;

const GLYPH_CHECK: u16 = 0xE73E;
const GLYPH_RADIO: u16 = 0xE915;
const GLYPH_CHEVRON: u16 = 0xE974;

pub const CORNERS_AT_PAINT: u32 = 1;
pub const CORNERS_AT_IDLE: u32 = 2;
pub const BORDER_COLOUR: u32 = 4;

pub fn scale(v: i32, dpi: u32) -> i32 {
    (v * dpi as i32 + 48) / 96
}

pub fn item_height(dpi: u32) -> i32 {
    scale(MARGIN_Y * 2 + PAD_TOP + LINE + PAD_BOTTOM, dpi)
}

pub fn separator_height(dpi: u32) -> i32 {
    scale(SEP_PAD * 2 + SEP_LINE, dpi)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    fn from_colorref(c: u32) -> Self {
        Rgb(c as u8, (c >> 8) as u8, (c >> 16) as u8)
    }
    fn colorref(self) -> COLORREF {
        COLORREF(self.0 as u32 | (self.1 as u32) << 8 | (self.2 as u32) << 16)
    }
    fn px(self) -> u32 {
        (self.0 as u32) << 16 | (self.1 as u32) << 8 | self.2 as u32
    }
    pub fn hex(self) -> String {
        format!("{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

/// WinUI colours are ARGB with alpha; the menu needs the solid colour they give over its background.
pub fn over(argb: u32, bg: Rgb) -> Rgb {
    let a = (argb >> 24) as f32 / 255.0;
    let mix = |c: u32, b: u8| (c as f32 * a + b as f32 * (1.0 - a)).round() as u8;
    Rgb(
        mix((argb >> 16) & 0xff, bg.0),
        mix((argb >> 8) & 0xff, bg.1),
        mix(argb & 0xff, bg.2),
    )
}

/// Linear interpolation from `bg` to `fg`, `t` in 0..=1.
fn lerp(bg: Rgb, fg: Rgb, t: f32) -> Rgb {
    let m = |b: u8, f: u8| (b as f32 + (f as f32 - b as f32) * t).round() as u8;
    Rgb(m(bg.0, fg.0), m(bg.1, fg.1), m(bg.2, fg.2))
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Rgb,
    pub text: Rgb,
    pub disabled: Rgb,
    pub hover: Rgb,
    pub hover_text: Rgb,
    pub separator: Rgb,
    pub border: Rgb,
}

pub fn palette(dark: bool, high_contrast: bool) -> Palette {
    if high_contrast {
        // The MenuFlyout high contrast dictionary maps to these system colours.
        let c = |i| Rgb::from_colorref(unsafe { GetSysColor(i) });
        return Palette {
            bg: c(COLOR_WINDOW),
            text: c(COLOR_BTNTEXT),
            disabled: c(COLOR_GRAYTEXT),
            hover: c(COLOR_HIGHLIGHT),
            hover_text: c(COLOR_HIGHLIGHTTEXT),
            separator: c(COLOR_WINDOWTEXT),
            border: c(COLOR_WINDOWTEXT),
        };
    }
    // No acrylic behind a classic menu window, so the fill is the solid colour the classic menu
    // already uses; the item colours are the WinUI tokens laid over it.
    if dark {
        let bg = Rgb(0x2c, 0x2c, 0x2c);
        Palette {
            bg,
            text: Rgb(0xff, 0xff, 0xff),
            disabled: over(0x5DFFFFFF, bg),
            hover: over(0x0FFFFFFF, bg),
            hover_text: Rgb(0xff, 0xff, 0xff),
            separator: over(0x15FFFFFF, bg),
            border: over(0x33000000, bg),
        }
    } else {
        let bg = Rgb(0xf9, 0xf9, 0xf9);
        let text = over(0xE4000000, bg);
        Palette {
            bg,
            text,
            disabled: over(0x5C000000, bg),
            hover: over(0x09000000, bg),
            hover_text: text,
            separator: over(0x0F000000, bg),
            border: over(0x0F000000, bg),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Text,
    Separator,
    Submenu,
}

/// One menu entry, used by both the classic and the owner-drawn builder.
pub struct Entry {
    pub text: String,
    pub id: u32,
    pub kind: Kind,
    pub checked: bool,
    pub grayed: bool,
    /// A check item: it gets the check column even while unchecked.
    pub toggle: bool,
    pub radio: bool,
    pub sub: Vec<Entry>,
}

impl Entry {
    pub fn text(id: u32, text: &str) -> Self {
        Entry {
            text: text.into(),
            id,
            kind: Kind::Text,
            checked: false,
            grayed: false,
            toggle: false,
            radio: false,
            sub: Vec::new(),
        }
    }
    pub fn separator() -> Self {
        Entry {
            kind: Kind::Separator,
            ..Entry::text(0, "")
        }
    }
}

// Screen readers find the item name through MSAAMENUINFO, which must be the first field of the item data.
#[repr(C)]
struct Item {
    msaa: MSAAMENUINFO,
    text: Vec<u16>,
    kind: Kind,
    radio: bool,
    check_col: bool,
}

struct Ctx {
    dpi: u32,
    pal: Palette,
    font: HFONT,
    glyphs: HFONT,
    brush: HBRUSH,
}

thread_local! {
    // Boxed so each item keeps its address while the Vec grows: Windows holds raw pointers to them.
    #[allow(clippy::vec_box)]
    static ITEMS: RefCell<Vec<Box<Item>>> = const { RefCell::new(Vec::new()) };
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
    static CORNERS: Cell<u32> = const { Cell::new(0) };
    static STYLED: RefCell<Vec<isize>> = const { RefCell::new(Vec::new()) };
}

fn new_item(e: &Entry, check_col: bool) -> Box<Item> {
    let mut text: Vec<u16> = e.text.encode_utf16().collect();
    let len = text.len() as u32;
    text.push(0);
    let mut item = Box::new(Item {
        msaa: MSAAMENUINFO {
            dwMSAASignature: MSAA_MENU_SIG as u32,
            cchWText: len,
            pszWText: PWSTR::null(),
        },
        text,
        kind: e.kind,
        radio: e.radio,
        check_col,
    });
    // The Vec's heap buffer does not move when the Box moves, so the pointer stays valid.
    item.msaa.pszWText = PWSTR(item.text.as_mut_ptr());
    item
}

fn has_check_col(entries: &[Entry]) -> bool {
    // WinUI reserves the check column for every item once any item in the menu can be checked.
    entries.iter().any(|e| e.radio || e.toggle)
}

/// Builds an owner-drawn menu. The item data lives until `end`.
pub fn build(entries: &[Entry]) -> HMENU {
    let menu = unsafe { CreatePopupMenu() }.expect("menu");
    let check_col = has_check_col(entries);
    for (pos, e) in entries.iter().enumerate() {
        let item = new_item(e, check_col);
        let data = &*item as *const Item as usize;
        ITEMS.with(|v| v.borrow_mut().push(item));
        let mut ftype = MFT_OWNERDRAW;
        if e.kind == Kind::Separator {
            ftype |= MFT_SEPARATOR;
        }
        if e.radio {
            ftype |= MFT_RADIOCHECK;
        }
        let mut state = MENU_ITEM_STATE(0);
        if e.checked {
            state |= MFS_CHECKED;
        }
        if e.grayed {
            state |= MFS_GRAYED;
        }
        let mut mii = MENUITEMINFOW {
            cbSize: size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_FTYPE | MIIM_ID | MIIM_STATE | MIIM_DATA,
            fType: ftype,
            fState: state,
            wID: e.id,
            dwItemData: data,
            ..Default::default()
        };
        if e.kind == Kind::Submenu {
            mii.fMask |= MIIM_SUBMENU;
            mii.hSubMenu = build(&e.sub);
        }
        unsafe {
            let _ = InsertMenuItemW(menu, pos as u32, true, &mii);
        }
    }
    menu
}

fn font(face: PCWSTR, px: i32) -> HFONT {
    unsafe {
        CreateFontW(
            -px,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            DEFAULT_QUALITY,
            0,
            face,
        )
    }
}

fn face_of(f: HFONT) -> String {
    unsafe {
        let dc = CreateCompatibleDC(None);
        let old = SelectObject(dc, f.into());
        let mut buf = [0u16; 64];
        GetTextFaceW(dc, Some(&mut buf));
        SelectObject(dc, old);
        let _ = DeleteDC(dc);
        let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..n])
    }
}

/// Sets up fonts and colours for one menu open and returns a log line.
pub fn begin(dpi: u32, dark: bool, high_contrast: bool) -> String {
    let pal = palette(dark, high_contrast);
    let text_font = font(w!("Segoe UI Variable Text"), scale(FONT_PX, dpi));
    let mut glyphs = font(w!("Segoe Fluent Icons"), scale(GLYPH, dpi));
    let mut glyph_face = face_of(glyphs);
    if glyph_face != "Segoe Fluent Icons" {
        unsafe {
            let _ = DeleteObject(glyphs.into());
        }
        glyphs = font(w!("Segoe MDL2 Assets"), scale(GLYPH, dpi));
        glyph_face = face_of(glyphs);
    }
    let line = format!(
        "owner-drawn menu: dpi {dpi}, dark {dark}, high contrast {high_contrast}, font '{}' {} px, glyphs '{glyph_face}' {} px, item {} px, separator {} px, bg {} text {} hover {} separator {} border {}",
        face_of(text_font),
        scale(FONT_PX, dpi),
        scale(GLYPH, dpi),
        item_height(dpi),
        separator_height(dpi),
        pal.bg.hex(),
        pal.text.hex(),
        pal.hover.hex(),
        pal.separator.hex(),
        pal.border.hex()
    );
    let brush = unsafe { CreateSolidBrush(pal.bg.colorref()) };
    CTX.with(|c| {
        *c.borrow_mut() = Some(Ctx {
            dpi,
            pal,
            font: text_font,
            glyphs,
            brush,
        })
    });
    line
}

/// Paints the menu gutter (above the first and below the last item) in our background colour.
pub fn set_background(menu: HMENU) {
    let brush = CTX.with(|c| c.borrow().as_ref().map(|c| c.brush));
    if let Some(b) = brush {
        let mi = MENUINFO {
            cbSize: size_of::<MENUINFO>() as u32,
            fMask: MIM_BACKGROUND | MIM_APPLYTOSUBMENUS,
            hbrBack: b,
            ..Default::default()
        };
        unsafe {
            let _ = SetMenuInfo(menu, &mi);
        }
    }
}

pub fn end() {
    if let Some(c) = CTX.with(|c| c.borrow_mut().take()) {
        unsafe {
            let _ = DeleteObject(c.font.into());
            let _ = DeleteObject(c.glyphs.into());
            let _ = DeleteObject(c.brush.into());
        }
    }
    ITEMS.with(|v| v.borrow_mut().clear());
    STYLED.with(|v| v.borrow_mut().clear());
}

pub fn dpi_at(x: i32, y: i32) -> u32 {
    let m = unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) };
    let (mut dx, mut dy) = (96u32, 96u32);
    let _ = unsafe { GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) };
    dx
}

pub fn work_area_at(x: i32, y: i32) -> RECT {
    let m = unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTOPRIMARY) };
    let mut mi = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = unsafe { GetMonitorInfoW(m, &mut mi) };
    mi.rcWork
}

/// The data of an item of ours, or None for anything else.
fn item_ref<'a>(data: usize) -> Option<&'a Item> {
    if data == 0 {
        return None;
    }
    let item = unsafe { &*(data as *const Item) };
    (item.msaa.dwMSAASignature == MSAA_MENU_SIG as u32).then_some(item)
}

fn text_width(item: &Item, f: HFONT) -> i32 {
    let mut text = item.text[..item.text.len() - 1].to_vec();
    let mut r = RECT::default();
    unsafe {
        let dc = CreateCompatibleDC(None);
        let old = SelectObject(dc, f.into());
        DrawTextW(dc, &mut text, &mut r, DT_CALCRECT | DT_SINGLELINE);
        SelectObject(dc, old);
        let _ = DeleteDC(dc);
    }
    r.right - r.left
}

fn item_width(item: &Item, dpi: u32, f: HFONT) -> i32 {
    let mut w = scale(MARGIN_X + PAD_X, dpi) * 2 + text_width(item, f);
    if item.check_col {
        w += scale(CHECK_COL, dpi);
    }
    if item.kind == Kind::Submenu {
        w += scale(CHEVRON_GAP + GLYPH, dpi);
    }
    w
}

pub fn measure(mis: &mut MEASUREITEMSTRUCT) -> bool {
    let Some(item) = item_ref(mis.itemData) else {
        return false;
    };
    CTX.with(|c| {
        let c = c.borrow();
        let Some(c) = c.as_ref() else { return false };
        if item.kind == Kind::Separator {
            mis.itemWidth = 0;
            mis.itemHeight = separator_height(c.dpi) as u32;
        } else {
            mis.itemWidth = item_width(item, c.dpi, c.font) as u32;
            mis.itemHeight = item_height(c.dpi) as u32;
        }
        log!(
            "WM_MEASUREITEM id {} {:?}: asked {} x {}",
            mis.itemID,
            item.kind,
            mis.itemWidth,
            mis.itemHeight
        );
        true
    })
}

/// Anti-aliased coverage of pixel (px, py) by a rounded rect: 1 inside, 0 outside, a fraction on the edge.
pub fn coverage(px: i32, py: i32, r: (i32, i32, i32, i32), radius: f32) -> f32 {
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

/// Background, hover shape and separator line, straight into the pixel buffer (0x00RRGGBB).
pub fn paint_shapes(px: &mut [u32], w: i32, h: i32, kind: Kind, hot: bool, dpi: u32, p: &Palette) {
    px.fill(p.bg.px());
    if kind == Kind::Separator {
        let y0 = scale(SEP_PAD, dpi);
        let y1 = (y0 + scale(SEP_LINE, dpi).max(1)).min(h);
        for y in y0..y1 {
            px[(y * w) as usize..((y + 1) * w) as usize].fill(p.separator.px());
        }
        return;
    }
    if !hot {
        return;
    }
    let r = (
        scale(MARGIN_X, dpi),
        scale(MARGIN_Y, dpi),
        w - scale(MARGIN_X, dpi),
        h - scale(MARGIN_Y, dpi),
    );
    let radius = scale(RADIUS, dpi) as f32;
    for y in r.1..r.3 {
        for x in r.0..r.2 {
            let a = coverage(x, y, r, radius);
            if a > 0.0 {
                px[(y * w + x) as usize] = lerp(p.bg, p.hover, a).px();
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct DrawState {
    pub hot: bool,
    pub disabled: bool,
    pub checked: bool,
    pub no_accel: bool,
}

struct Dib {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u32,
    w: i32,
    h: i32,
}

impl Dib {
    fn new(w: i32, h: i32) -> Option<Dib> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        unsafe {
            let dc = CreateCompatibleDC(None);
            let bmp = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
            let old = SelectObject(dc, bmp.into());
            Some(Dib {
                dc,
                bmp,
                old,
                bits: bits.cast(),
                w,
                h,
            })
        }
    }
    fn pixels(&mut self) -> &mut [u32] {
        unsafe {
            // GDI may still hold queued drawing for this bitmap; the raw pixels must see it.
            let _ = GdiFlush();
            std::slice::from_raw_parts_mut(self.bits, (self.w * self.h) as usize)
        }
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bmp.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

fn draw_text(dc: HDC, f: HFONT, text: &[u16], mut r: RECT, colour: Rgb, fmt: DRAW_TEXT_FORMAT) {
    let mut t = text.to_vec();
    unsafe {
        let old = SelectObject(dc, f.into());
        SetTextColor(dc, colour.colorref());
        DrawTextW(
            dc,
            &mut t,
            &mut r,
            fmt | DT_SINGLELINE | DT_VCENTER | DT_NOCLIP,
        );
        SelectObject(dc, old);
    }
}

/// Draws one item into `dst` at `rc`, through an off-screen bitmap so nothing flickers.
fn render(dst: HDC, rc: RECT, item: &Item, st: DrawState, c: &Ctx) {
    let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
    let Some(mut dib) = Dib::new(w, h) else {
        return;
    };
    let p = &c.pal;
    paint_shapes(dib.pixels(), w, h, item.kind, st.hot, c.dpi, p);
    if item.kind != Kind::Separator {
        let colour = if st.disabled {
            p.disabled
        } else if st.hot {
            p.hover_text
        } else {
            p.text
        };
        let s = |v| scale(v, c.dpi);
        let left = s(MARGIN_X + PAD_X);
        let top = s(MARGIN_Y + PAD_TOP);
        let bottom = top + s(LINE);
        unsafe { SetBkMode(dib.dc, TRANSPARENT) };
        let text_left = left + if item.check_col { s(CHECK_COL) } else { 0 };
        let accel = if st.no_accel {
            DT_HIDEPREFIX
        } else {
            DRAW_TEXT_FORMAT(0)
        };
        let text = &item.text[..item.text.len() - 1];
        let r = RECT {
            left: text_left,
            top,
            right: w - left,
            bottom,
        };
        draw_text(dib.dc, c.font, text, r, colour, DT_LEFT | accel);
        if st.checked {
            let g = if item.radio { GLYPH_RADIO } else { GLYPH_CHECK };
            let r = RECT {
                left,
                top,
                right: left + s(GLYPH),
                bottom,
            };
            draw_text(dib.dc, c.glyphs, &[g], r, colour, DT_CENTER);
        }
        if item.kind == Kind::Submenu {
            let r = RECT {
                left: w - left - s(GLYPH),
                top,
                right: w - left,
                bottom,
            };
            draw_text(dib.dc, c.glyphs, &[GLYPH_CHEVRON], r, colour, DT_CENTER);
        }
    }
    unsafe {
        let _ = BitBlt(dst, rc.left, rc.top, w, h, Some(dib.dc), 0, 0, SRCCOPY);
    }
}

pub fn draw(dis: &DRAWITEMSTRUCT) -> bool {
    let Some(item) = item_ref(dis.itemData) else {
        return false;
    };
    let s = dis.itemState.0;
    let st = DrawState {
        hot: s & ODS_SELECTED.0 != 0,
        disabled: s & (ODS_GRAYED.0 | ODS_DISABLED.0) != 0,
        checked: s & ODS_CHECKED.0 != 0,
        no_accel: s & ODS_NOACCEL.0 != 0,
    };
    let rc = dis.rcItem;
    if CORNERS.with(Cell::get) & CORNERS_AT_PAINT != 0 {
        style_once(unsafe { WindowFromDC(dis.hDC) }, "first paint");
    }
    let done = CTX.with(|c| {
        let c = c.borrow();
        let Some(c) = c.as_ref() else { return false };
        render(dis.hDC, rc, item, st, c);
        true
    });
    if done && item.kind == Kind::Submenu {
        // Windows paints its own submenu arrow after WM_DRAWITEM; clipping the item keeps ours alone.
        unsafe {
            ExcludeClipRect(dis.hDC, rc.left, rc.top, rc.right, rc.bottom);
        }
    }
    let name = String::from_utf16_lossy(&item.text[..item.text.len() - 1]);
    log!(
        "WM_DRAWITEM id {} '{name}' state 0x{s:x} (hot {}, disabled {}, checked {}, no accel {}) rc {}x{} at {},{}",
        dis.itemID,
        st.hot,
        st.disabled,
        st.checked,
        st.no_accel,
        rc.right - rc.left,
        rc.bottom - rc.top,
        rc.left,
        rc.top
    );
    done
}

/// The mnemonic after a single '&', lower-cased.
pub fn mnemonic(text: &[u16]) -> Option<char> {
    let s = String::from_utf16_lossy(text);
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '&' {
            match it.next() {
                Some('&') => continue,
                Some(m) => return m.to_lowercase().next(),
                None => return None,
            }
        }
    }
    None
}

/// Owner-drawn items have no text Windows can match, so mnemonics come here.
pub fn menu_char(wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    let ch = char::from_u32((wp.0 & 0xffff) as u32)?
        .to_lowercase()
        .next()?;
    let menu = HMENU(lp.0 as *mut c_void);
    let n = unsafe { GetMenuItemCount(Some(menu)) };
    for pos in 0..n.max(0) as u32 {
        let mut mii = MENUITEMINFOW {
            cbSize: size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_DATA | MIIM_STATE,
            ..Default::default()
        };
        if unsafe { GetMenuItemInfoW(menu, pos, true, &mut mii) }.is_err() {
            continue;
        }
        let Some(item) = item_ref(mii.dwItemData) else {
            continue;
        };
        if mii.fState.0 & MFS_GRAYED.0 == 0 && mnemonic(&item.text) == Some(ch) {
            log!("WM_MENUCHAR '{ch}': item at {pos}, MNC_EXECUTE");
            return Some(LRESULT((MNC_EXECUTE as isize) << 16 | pos as isize));
        }
    }
    log!("WM_MENUCHAR '{ch}': no item, default handling");
    None
}

fn style_window(h: HWND, when: &str) {
    let bits = CORNERS.with(Cell::get);
    let pref = DWMWCP_ROUND;
    let round = unsafe {
        DwmSetWindowAttribute(
            h,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &pref as *const _ as *const c_void,
            size_of_val(&pref) as u32,
        )
    };
    let mut border = String::from("not set");
    if bits & BORDER_COLOUR != 0 {
        let colour = CTX.with(|c| c.borrow().as_ref().map(|c| c.pal.border));
        if let Some(b) = colour {
            let cr = b.colorref();
            let r = unsafe {
                DwmSetWindowAttribute(
                    h,
                    DWMWA_BORDER_COLOR,
                    &cr as *const _ as *const c_void,
                    size_of_val(&cr) as u32,
                )
            };
            border = format!("{} {r:?}", b.hex());
        }
    }
    log!("menu window {h:?} at {when}: DWMWCP_ROUND {round:?}, border colour {border}");
}

// The earliest point that works: at HCBT_CREATEWND the handle is too new for DWM ("handle is invalid"), and
// call hooks never see the menu window's WM_CREATE (seen 2026-11-03). The first WM_DRAWITEM comes during the
// window's first paint, before WM_ENTERIDLE.
pub fn hook_on(bits: u32) {
    CORNERS.with(|c| c.set(bits));
}

pub fn hook_off() {
    CORNERS.with(|c| c.set(0));
}

fn style_once(h: HWND, when: &str) {
    if h.is_invalid() {
        return;
    }
    let first = STYLED.with(|v| {
        let mut v = v.borrow_mut();
        let key = h.0 as isize;
        let first = !v.contains(&key);
        if first {
            v.push(key);
        }
        first
    });
    if first {
        style_window(h, when);
    }
}

/// WM_ENTERIDLE names the menu window once it shows: a later second chance.
pub fn on_idle(menu_window: HWND) {
    if CORNERS.with(Cell::get) & CORNERS_AT_IDLE != 0 {
        style_once(menu_window, "idle");
    }
}

/// `--render`: draws the menus at several DPIs and palettes into PNGs, with no window and no screen.
pub fn render_all(menus: &[(&str, Vec<Entry>, usize)], out: &Path) {
    let _ = std::fs::create_dir_all(out);
    for dpi in [96u32, 120, 144] {
        for (theme, dark, hc) in [
            ("light", false, false),
            ("dark", true, false),
            ("hc", false, true),
        ] {
            let line = begin(dpi, dark, hc);
            log!("render: {line}");
            for (name, entries, hot) in menus {
                let check_col = has_check_col(entries);
                let items: Vec<Box<Item>> =
                    entries.iter().map(|e| new_item(e, check_col)).collect();
                let file = out.join(format!("render-{dpi}dpi-{theme}-{name}.png"));
                let r = CTX.with(|c| {
                    let c = c.borrow();
                    let c = c.as_ref().expect("begin ran");
                    render_menu(&items, entries, *hot, c, &file)
                });
                log!("render {}: {r:?}", file.display());
            }
            end();
        }
    }
}

fn render_menu(
    items: &[Box<Item>],
    entries: &[Entry],
    hot: usize,
    c: &Ctx,
    file: &Path,
) -> Result<String, String> {
    let width = items
        .iter()
        .filter(|i| i.kind != Kind::Separator)
        .map(|i| item_width(i, c.dpi, c.font))
        .max()
        .unwrap_or(0);
    let heights: Vec<i32> = items
        .iter()
        .map(|i| {
            if i.kind == Kind::Separator {
                separator_height(c.dpi)
            } else {
                item_height(c.dpi)
            }
        })
        .collect();
    let total: i32 = heights.iter().sum();
    let mut canvas = Dib::new(width, total).ok_or("canvas")?;
    let mut y = 0;
    for (i, (item, e)) in items.iter().zip(entries).enumerate() {
        let rc = RECT {
            left: 0,
            top: y,
            right: width,
            bottom: y + heights[i],
        };
        let st = DrawState {
            hot: i == hot,
            disabled: e.grayed,
            checked: e.checked,
            no_accel: false,
        };
        render(canvas.dc, rc, item, st, c);
        y += heights[i];
    }
    let rgba: Vec<u8> = canvas
        .pixels()
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255])
        .collect();
    crate::capture::write_png(file, width as u32, total as u32, &rgba)?;
    Ok(format!("{width} x {total}, item {} px", item_height(c.dpi)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_follow_the_winui_template() {
        assert_eq!(item_height(96), 33);
        assert_eq!(item_height(120), 41);
        assert_eq!(item_height(144), 50);
        assert_eq!(separator_height(96), 3);
        assert_eq!(separator_height(120), 4);
    }

    #[test]
    fn winui_tokens_over_the_menu_fill() {
        let light = palette(false, false);
        assert_eq!(light.text, Rgb(0x1a, 0x1a, 0x1a));
        assert_eq!(light.hover, Rgb(0xf0, 0xf0, 0xf0));
        assert_eq!(light.disabled, Rgb(0x9f, 0x9f, 0x9f));
        assert_eq!(light.separator, Rgb(0xea, 0xea, 0xea));
        let dark = palette(true, false);
        assert_eq!(dark.hover, Rgb(0x38, 0x38, 0x38));
        assert_eq!(dark.disabled, Rgb(0x79, 0x79, 0x79));
        assert_eq!(dark.separator, Rgb(0x3d, 0x3d, 0x3d));
    }

    #[test]
    fn coverage_is_full_inside_empty_outside_and_soft_at_corners() {
        let r = (4, 2, 100, 31);
        assert_eq!(coverage(50, 15, r, 4.0), 1.0);
        assert_eq!(coverage(3, 15, r, 4.0), 0.0);
        assert_eq!(coverage(50, 31, r, 4.0), 0.0);
        assert_eq!(coverage(4, 2, r, 4.0), 0.0, "outside the arc");
        let arc = coverage(4, 4, r, 4.0);
        assert!(arc > 0.0 && arc < 1.0, "pixel on the arc {arc}");
    }

    #[test]
    fn mnemonic_skips_double_ampersand() {
        let t = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
        assert_eq!(mnemonic(&t("&Keep awake")), Some('k'));
        assert_eq!(mnemonic(&t("Save && &Quit")), Some('q'));
        assert_eq!(mnemonic(&t("Automatic")), None);
    }

    #[test]
    fn hover_shape_is_inset_and_rounded() {
        let p = palette(false, false);
        let (w, h) = (200, item_height(96));
        let mut px = vec![0u32; (w * h) as usize];
        paint_shapes(&mut px, w, h, Kind::Text, true, 96, &p);
        let at = |x: i32, y: i32| px[(y * w + x) as usize];
        assert_eq!(at(1, 15), p.bg.px(), "left margin keeps the fill");
        assert_eq!(at(100, 15), p.hover.px(), "middle is hover");
        assert_eq!(at(100, 0), p.bg.px(), "top margin keeps the fill");
        assert_ne!(at(4, 2), p.hover.px(), "corner is rounded");
    }

    #[test]
    fn render_draws_text_in_a_bitmap_without_a_screen() {
        let menus = [("main", vec![Entry::text(1, "&Automatic")], 0usize)];
        let out = std::env::temp_dir().join("wakeshot-look-test");
        render_all(&menus, &out);
        let f = out.join("render-120dpi-dark-main.png");
        assert!(f.exists() || cfg!(not(feature = "screenshots")));
        let _ = std::fs::remove_dir_all(&out);
    }
}
