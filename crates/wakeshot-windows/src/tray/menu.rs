//! `OwnerDrawnMenu`, the `MenuView` the product uses.
//!
//! Owner: `p5-menu-view`.
//!
//! The Phase 1 pick (`docs/research/tray-menu-look.md`, "Pick"): a classic Win32 popup menu, so Windows keeps the menu
//! loop, keyboard, access keys, light dismiss, placement, submenus and screen readers, with every item owner-drawn by
//! `menu_paint` and each menu window rounded by DWM. Every open builds a fresh `HMENU` with the theme and the DPI of
//! that moment, so a theme switch shows at the next open without a restart.
//!
//! The state of one open sits in a `RefCell` that the hidden owner window reaches through an `Rc`. `show` holds no
//! borrow while `TrackPopupMenuEx` runs its modal loop, which sends the owner-draw messages to that window.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Arc;

use wakeshot_core::event::Event;
use wakeshot_core::view::{MenuCommand, MenuItem, MenuModel};
use windows::Win32::Foundation::{
    COLORREF, ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT,
    WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DWMWINDOWATTRIBUTE,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, ExcludeClipRect, GetMonitorInfoW, HBRUSH,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, WindowFromDC,
};
use windows::Win32::System::LibraryLoader::{
    GetModuleHandleW, GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows::Win32::UI::Accessibility::{MSAA_MENU_SIG, MSAAMENUINFO};
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODT_MENU};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForSystem, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, GWLP_USERDATA,
    GetSystemMetrics, GetWindowLongPtrW, HMENU, InsertMenuItemW, MENUINFO, MENUITEMINFOW,
    MFS_CHECKED, MFS_GRAYED, MFT_OWNERDRAW, MFT_RADIOCHECK, MFT_SEPARATOR, MIIM_DATA, MIIM_FTYPE,
    MIIM_ID, MIIM_STATE, MIIM_SUBMENU, MIM_APPLYTOSUBMENUS, MIM_BACKGROUND, MNC_EXECUTE,
    MNC_IGNORE, MSGF_MENU, PostMessageW, RegisterClassExW, SM_MENUDROPALIGNMENT,
    SetForegroundWindow, SetMenuInfo, SetWindowLongPtrW, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TPM_RETURNCMD, TPM_RIGHTALIGN, TPM_RIGHTBUTTON, TPM_TOPALIGN, TRACK_POPUP_MENU_FLAGS,
    TrackPopupMenuEx, WM_CLOSE, WM_DRAWITEM, WM_ENTERIDLE, WM_MEASUREITEM, WM_MENUCHAR, WM_NULL,
    WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};
use windows::core::{PCSTR, PCWSTR, PWSTR, w};

use super::menu_paint::{self, Columns, Fonts, ItemState, Layout, Mark, Palette};
use super::theme::{self, Theme};
use super::{Anchor, MenuView};
use crate::WinError;
use crate::sink::EventSink;

const CLASS: PCWSTR = w!("Wakeshot.TrayMenu");

/// What a failed theme read draws, as the icon does; the next open reads again.
const FALLBACK_THEME: Theme = Theme {
    light: false,
    contrast: None,
};

/// What the menu did, for the log and the demo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// The menu opened at `dpi` with `theme`. `foreground`: what `SetForegroundWindow` returned; without it the menu
    /// may not close on an outside click. `frame`: the classic frame got the same mode (uxtheme had both calls).
    Opened {
        dpi: u32,
        theme: Theme,
        foreground: bool,
        frame: bool,
    },
    /// A menu window got round corners and the border colour.
    Styled {
        at: StyledAt,
        corners: Result<(), WinError>,
        border: Result<(), WinError>,
    },
    /// The menu closed. `command`: the pick, `None` on Esc or an outside click.
    Closed { command: Option<MenuCommand> },
    /// Building the menu failed (no menu opened), an item did not paint, or the pick could not be posted (it is lost).
    Failed(WinError),
}

/// When a menu window got its DWM attributes: at its first item paint, or at `WM_ENTERIDLE` when no paint came first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyledAt {
    FirstPaint,
    Idle,
}

/// Called with each `Note`.
pub type NoteHook = Box<dyn FnMut(&Note)>;

/// The tray menu, on the loop thread only (`Rc` keeps it off other threads).
pub struct OwnerDrawnMenu {
    hwnd: HWND,
    shared: Rc<Shared>,
    sink: Arc<dyn EventSink>,
    ux: UxTheme,
    forced_theme: Option<Theme>,
}

impl OwnerDrawnMenu {
    /// Makes the hidden owner window that `TrackPopupMenuEx` needs and that gets the owner-draw messages. Call it on
    /// the loop thread: the menu's windows belong to the thread that opens it.
    pub fn new(sink: Arc<dyn EventSink>) -> Result<Self, WinError> {
        // SAFETY: plain call; `None` names this exe.
        let module =
            unsafe { GetModuleHandleW(None) }.map_err(|e| win_error("GetModuleHandleW", &e))?;
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: module.into(),
            lpszClassName: CLASS,
            ..Default::default()
        };
        // SAFETY: `wc` and the class name it points at outlive the call.
        if unsafe { RegisterClassExW(&wc) } == 0 {
            // SAFETY: no arguments.
            let error = unsafe { GetLastError() };
            // A second menu in this process (tests, a demo) finds the class registered.
            if error != ERROR_CLASS_ALREADY_EXISTS {
                return Err(WinError {
                    call: "RegisterClassExW",
                    code: error.0,
                });
            }
        }
        // Top-level, not message-only: only a top-level window can be the foreground window, and a tray menu whose
        // owner is not the foreground window does not close on an outside click (research, "Menu details").
        // SAFETY: the class is registered above; the window is never shown and is destroyed in `Drop`, on this thread.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                CLASS,
                w!("Wakeshot tray menu"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(module.into()),
                None,
            )
        }
        .map_err(|e| win_error("CreateWindowExW", &e))?;
        let shared = Rc::new(Shared {
            open: RefCell::new(None),
            note_hook: RefCell::new(None),
        });
        // SAFETY: the window made above; `Drop` clears the slot before this `Rc` can go.
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, Rc::as_ptr(&shared) as isize) };
        Ok(Self {
            hwnd,
            shared,
            sink,
            ux: UxTheme::load(),
            forced_theme: None,
        })
    }

    /// Called with each thing the menu did (opened, styled, closed, failed).
    pub fn on_note(&mut self, hook: NoteHook) {
        *self.shared.note_hook.borrow_mut() = Some(hook);
    }

    /// Demos only (`ux_menu --contrast`): open with this theme instead of the live one; `None` goes back to it.
    pub fn force_theme(&mut self, theme: Option<Theme>) {
        self.forced_theme = theme;
    }
}

impl MenuView for OwnerDrawnMenu {
    fn show(&mut self, menu: &MenuModel, anchor: Anchor) {
        if menu.items.is_empty() {
            return;
        }
        let theme = self
            .forced_theme
            .unwrap_or_else(|| theme::read().unwrap_or(FALLBACK_THEME));
        let (dpi, monitor) = monitor_at(anchor);
        // Items are ours, but Windows still draws the frame and the gutter around them with the process's mode.
        let frame = self.ux.set(mode_for(&theme));
        let open = match Open::new(&menu.items, &theme, dpi) {
            Ok(open) => open,
            Err(e) => {
                self.shared.note(&Note::Failed(e));
                return;
            }
        };
        let root = open.root;
        *self.shared.open.borrow_mut() = Some(open);
        // SAFETY: this thread's own window.
        let foreground = unsafe { SetForegroundWindow(self.hwnd) }.as_bool();
        self.shared.note(&Note::Opened {
            dpi,
            theme,
            foreground,
            frame,
        });
        // SAFETY: plain call.
        let drop_right = unsafe { GetSystemMetrics(SM_MENUDROPALIGNMENT) } != 0;
        // No `TPM_NONOTIFY`: the menu must send `WM_MENUCHAR` and the owner-draw messages.
        let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON | alignment(drop_right, anchor, monitor);
        // SAFETY: `root` lives in `shared.open` until after the call, and no borrow of it is held during the call.
        let id = unsafe { TrackPopupMenuEx(root, flags.0, anchor.x, anchor.y, self.hwnd, None) };
        // `TrackPopupMenuEx` Remarks: a message to the owner after the menu, so the next one also closes as it should.
        // SAFETY: this thread's own window.
        let _ = unsafe { PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0)) };
        let Some(mut open) = self.shared.open.borrow_mut().take() else {
            return;
        };
        let commands = std::mem::take(&mut open.commands);
        // The menu, its brush and fonts go before the pick is posted.
        drop(open);
        match post_pick(self.sink.as_ref(), &commands, id.0 as u32) {
            Ok(command) => self.shared.note(&Note::Closed { command }),
            Err(e) => self.shared.note(&Note::Failed(e)),
        }
    }
}

impl Drop for OwnerDrawnMenu {
    fn drop(&mut self) {
        // SAFETY: this thread's own window. The slot is cleared first, so the messages `DestroyWindow` sends never
        // see the state.
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

struct Shared {
    open: RefCell<Option<Open>>,
    note_hook: RefCell<Option<NoteHook>>,
}

impl Shared {
    fn note(&self, note: &Note) {
        // A note raised from inside the hook finds it borrowed and is dropped instead of panicking.
        if let Ok(mut hook) = self.note_hook.try_borrow_mut()
            && let Some(hook) = hook.as_mut()
        {
            hook(note);
        }
    }

    fn handle(&self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_MEASUREITEM => {
                // SAFETY: for this message `lparam` points at a live MEASUREITEMSTRUCT for the call.
                let mis = unsafe { &mut *(lparam.0 as *mut MEASUREITEMSTRUCT) };
                if mis.CtlType == ODT_MENU && self.measure(mis) {
                    return LRESULT(1);
                }
            }
            WM_DRAWITEM => {
                // SAFETY: for this message `lparam` points at a live DRAWITEMSTRUCT for the call.
                let dis = unsafe { &*(lparam.0 as *const DRAWITEMSTRUCT) };
                if dis.CtlType == ODT_MENU && self.draw(dis) {
                    return LRESULT(1);
                }
            }
            // Owner-drawn items have no text Windows can match a key against.
            WM_MENUCHAR => {
                if let Some(result) = self.menu_char(wparam, lparam) {
                    return LRESULT(result);
                }
            }
            WM_ENTERIDLE if wparam.0 as u32 == MSGF_MENU => {
                self.style(HWND(lparam.0 as *mut c_void), StyledAt::Idle);
            }
            // The loop window owns quitting; the default would destroy this window under a live menu view.
            WM_CLOSE => return LRESULT(0),
            _ => {}
        }
        // SAFETY: the default handling, with the message as it came.
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    fn measure(&self, mis: &mut MEASUREITEMSTRUCT) -> bool {
        let open = self.open.borrow();
        let Some(open) = open.as_ref() else {
            return false;
        };
        let Some(data) = open.data(mis.itemData) else {
            return false;
        };
        let (width, height) = menu_paint::measure(&data.item, &open.fonts, &data.layout);
        mis.itemWidth = width;
        mis.itemHeight = height;
        true
    }

    fn draw(&self, dis: &DRAWITEMSTRUCT) -> bool {
        let open = self.open.borrow();
        let Some(open) = open.as_ref() else {
            return false;
        };
        let Some(data) = open.data(dis.itemData) else {
            return false;
        };
        // The earliest point DWM takes the menu window: at `HCBT_CREATEWND` it answered "handle is invalid" (Phase 1).
        // SAFETY: the DC Windows gave for this paint.
        self.style_with(open, unsafe { WindowFromDC(dis.hDC) }, StyledAt::FirstPaint);
        let state = ItemState::from_ods(dis.itemState.0);
        match menu_paint::paint_item(
            dis.hDC,
            dis.rcItem,
            &data.item,
            state,
            &open.palette,
            &open.fonts,
            &data.layout,
        ) {
            Ok(Some(clip)) => {
                // Windows paints its own submenu arrow after this message; clipping the item keeps ours alone.
                // SAFETY: the DC Windows gave for this paint.
                unsafe { ExcludeClipRect(dis.hDC, clip.left, clip.top, clip.right, clip.bottom) };
            }
            Ok(None) => {}
            Err(e) => self.note(&Note::Failed(e)),
        }
        true
    }

    fn menu_char(&self, wparam: WPARAM, lparam: LPARAM) -> Option<isize> {
        let open = self.open.borrow();
        let open = open.as_ref()?;
        let menu = HMENU(lparam.0 as *mut c_void);
        let (_, positions) = open.menus.iter().find(|(m, _)| *m == menu)?;
        let key = (wparam.0 & 0xffff) as u16;
        Some(menu_char(
            key,
            positions.iter().map(|&i| &open.items[i].item),
        ))
    }

    fn style(&self, window: HWND, at: StyledAt) {
        let open = self.open.borrow();
        if let Some(open) = open.as_ref() {
            self.style_with(open, window, at);
        }
    }

    /// Once per menu window: each open, and each submenu, makes a new one.
    fn style_with(&self, open: &Open, window: HWND, at: StyledAt) {
        if window.is_invalid() {
            return;
        }
        {
            let mut styled = open.styled.borrow_mut();
            let key = window.0 as isize;
            if styled.contains(&key) {
                return;
            }
            styled.push(key);
        }
        // Without the call an owner-drawn menu loses Windows' rounding and gets a light square border (Phase 1).
        let corners = set_attribute(window, DWMWA_WINDOW_CORNER_PREFERENCE, &DWMWCP_ROUND);
        let border = set_attribute(
            window,
            DWMWA_BORDER_COLOR,
            &COLORREF(open.palette.border.colorref()),
        );
        self.note(&Note::Styled {
            at,
            corners,
            border,
        });
    }
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: plain read of this window's own slot.
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const Shared;
    if ptr.is_null() {
        // SAFETY: the default handling, with the message as it came.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // SAFETY: a non-null slot holds `Rc::as_ptr` of the `OwnerDrawnMenu` that owns this window, whose `Drop` clears it
    // before that `Rc` can go. A count of our own keeps the state alive to the end of this call.
    let shared = unsafe {
        Rc::increment_strong_count(ptr);
        Rc::from_raw(ptr)
    };
    shared.handle(hwnd, msg, wparam, lparam)
}

/// One open menu: the `HMENU` tree and everything its items draw with, freed together on drop.
struct Open {
    root: HMENU,
    palette: Palette,
    fonts: Fonts,
    brush: HBRUSH,
    /// Every item's data, depth first, made before the first insert and never resized: Windows holds a pointer to
    /// each.
    items: Box<[ItemData]>,
    /// Each `HMENU` with its items' places in `items`, in menu order, for `WM_MENUCHAR`.
    menus: Vec<(HMENU, Vec<usize>)>,
    commands: Vec<MenuCommand>,
    /// Menu windows (as `isize`) that have their DWM attributes.
    styled: RefCell<Vec<isize>>,
}

/// Where `Open::fill` is in its depth-first walk: the next item's place in `Open::items`, and the last id given.
#[derive(Default)]
struct Walk {
    item: usize,
    id: u32,
}

impl Open {
    fn new(items: &[MenuItem], theme: &Theme, dpi: u32) -> Result<Self, WinError> {
        let fonts = Fonts::new(dpi)?;
        let palette = Palette::for_theme(theme);
        let mut data = Vec::new();
        collect(items, dpi, &mut data);
        // SAFETY: plain call; the menu is destroyed when `Open` drops.
        let root = unsafe { CreatePopupMenu() }.map_err(|e| win_error("CreatePopupMenu", &e))?;
        // From here an early return drops `open`, which destroys the root and every submenu attached to it.
        let mut open = Self {
            root,
            palette,
            fonts,
            brush: HBRUSH::default(),
            items: data.into_boxed_slice(),
            menus: Vec::new(),
            commands: command_ids(items),
            styled: RefCell::new(Vec::new()),
        };
        // SAFETY: plain call; the brush is deleted when `Open` drops, after the menu that uses it.
        open.brush = unsafe { CreateSolidBrush(COLORREF(palette.background.colorref())) };
        if open.brush.is_invalid() {
            return Err(last_error("CreateSolidBrush"));
        }
        open.fill(root, items, &mut Walk::default())?;
        // The gutter above the first and below the last item, in the items' own fill.
        let info = MENUINFO {
            cbSize: size_of::<MENUINFO>() as u32,
            fMask: MIM_BACKGROUND | MIM_APPLYTOSUBMENUS,
            hbrBack: open.brush,
            ..Default::default()
        };
        // SAFETY: `info` is a live MENUINFO with `cbSize` set; the menu is ours.
        unsafe { SetMenuInfo(root, &info) }.map_err(|e| win_error("SetMenuInfo", &e))?;
        Ok(open)
    }

    /// Adds `items` to `menu` in the walk `collect` and `command_ids` take, so each item finds its data and each
    /// action's id is its place in `commands` plus one.
    fn fill(&mut self, menu: HMENU, items: &[MenuItem], walk: &mut Walk) -> Result<(), WinError> {
        let mut positions = Vec::with_capacity(items.len());
        for (pos, item) in items.iter().enumerate() {
            let index = walk.item;
            walk.item += 1;
            let mut mii = MENUITEMINFOW {
                cbSize: size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_FTYPE | MIIM_STATE | MIIM_ID | MIIM_DATA,
                fType: MFT_OWNERDRAW,
                dwItemData: std::ptr::from_ref::<ItemData>(&self.items[index]) as usize,
                ..Default::default()
            };
            let mut submenu = None;
            match item {
                MenuItem::Separator => mii.fType |= MFT_SEPARATOR,
                // A status line: Enter on it does nothing, and screen readers hear it as unavailable.
                MenuItem::Label(_) => mii.fState = MFS_GRAYED,
                MenuItem::Action {
                    command,
                    checked,
                    enabled,
                    ..
                } => {
                    walk.id += 1;
                    mii.wID = walk.id;
                    if menu_paint::mark_for(*command) == Some(Mark::Radio) {
                        mii.fType |= MFT_RADIOCHECK;
                    }
                    if *checked {
                        mii.fState |= MFS_CHECKED;
                    }
                    if !*enabled {
                        mii.fState |= MFS_GRAYED;
                    }
                }
                MenuItem::Submenu { items: inner, .. } => {
                    // SAFETY: plain call; destroyed below on failure, else with the root it is attached to.
                    let child = unsafe { CreatePopupMenu() }
                        .map_err(|e| win_error("CreatePopupMenu", &e))?;
                    if let Err(e) = self.fill(child, inner, walk) {
                        // SAFETY: made above and attached to nothing.
                        let _ = unsafe { DestroyMenu(child) };
                        return Err(e);
                    }
                    mii.fMask |= MIIM_SUBMENU;
                    mii.hSubMenu = child;
                    submenu = Some(child);
                }
            }
            positions.push(index);
            // SAFETY: `mii` is a live MENUITEMINFOW with `cbSize` set; its item data lives in `self.items` until the
            // menu is destroyed.
            if let Err(e) = unsafe { InsertMenuItemW(menu, pos as u32, true, &mii) } {
                if let Some(child) = submenu {
                    // SAFETY: made above; the insert that would attach it failed.
                    let _ = unsafe { DestroyMenu(child) };
                }
                return Err(win_error("InsertMenuItemW", &e));
            }
        }
        self.menus.push((menu, positions));
        Ok(())
    }

    /// The item data behind a `dwItemData` value, only when it is one of ours: nothing else is read through it.
    fn data(&self, item_data: usize) -> Option<&ItemData> {
        self.items
            .iter()
            .find(|d| std::ptr::from_ref::<ItemData>(d) as usize == item_data)
    }
}

/// Every item's data in the depth-first order `Open::fill` meets the items, each with its own menu's layout.
fn collect(items: &[MenuItem], dpi: u32, out: &mut Vec<ItemData>) {
    let layout = Layout::new(dpi, Columns::of(items));
    for item in items {
        out.push(ItemData::new(item, layout));
        if let MenuItem::Submenu { items: inner, .. } = item {
            collect(inner, dpi, out);
        }
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        // SAFETY: made in `new`; `DestroyMenu` also destroys every submenu, and the brush goes after the menu.
        unsafe {
            let _ = DestroyMenu(self.root);
            if !self.brush.is_invalid() {
                let _ = DeleteObject(self.brush.into());
            }
        }
    }
}

/// What `dwItemData` points at. `msaa` comes first, where screen readers look for it (`MSAAMENUINFO` Remarks).
#[repr(C)]
struct ItemData {
    msaa: MSAAMENUINFO,
    /// The menu text as given, access key marks included, with an end mark. Windows' accessibility proxy drops the
    /// marks itself: Phase 1 gave it "&Keep awake" and the screen reader read "Keep awake". Stripping them here too
    /// would drop a literal `&` (a status line's `&&`).
    text: Vec<u16>,
    item: MenuItem,
    layout: Layout,
}

impl ItemData {
    fn new(item: &MenuItem, layout: Layout) -> Self {
        let mut text: Vec<u16> = text_of(item).encode_utf16().collect();
        let len = text.len() as u32;
        text.push(0);
        Self {
            msaa: MSAAMENUINFO {
                dwMSAASignature: MSAA_MENU_SIG as u32,
                cchWText: len,
                // The text's heap buffer stays where it is when the data moves.
                pszWText: PWSTR(text.as_mut_ptr()),
            },
            text,
            item: item.clone(),
            layout,
        }
    }
}

fn text_of(item: &MenuItem) -> &str {
    match item {
        MenuItem::Label(t)
        | MenuItem::Action { text: t, .. }
        | MenuItem::Submenu { text: t, .. } => t,
        MenuItem::Separator => "",
    }
}

/// The command table of one menu: each action's command in the order `Open::fill` meets it (depth first, a
/// submenu's items at the submenu's place). An action's id is its place here plus one, since 0 is what
/// `TrackPopupMenuEx` returns when nothing was picked. Built per menu, so any `Minutes(n)` gets an id.
fn command_ids(items: &[MenuItem]) -> Vec<MenuCommand> {
    fn walk(items: &[MenuItem], out: &mut Vec<MenuCommand>) {
        for item in items {
            match item {
                MenuItem::Action { command, .. } => out.push(*command),
                MenuItem::Submenu { items, .. } => walk(items, out),
                MenuItem::Label(_) | MenuItem::Separator => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(items, &mut out);
    out
}

/// The command behind `id`; `None` for 0 (Esc, an outside click) and for an id no item has.
fn command_for(commands: &[MenuCommand], id: u32) -> Option<MenuCommand> {
    let index = usize::try_from(id.checked_sub(1)?).ok()?;
    commands.get(index).copied()
}

/// Posts the pick as `Event::Menu`; nothing for an id of no command.
fn post_pick(
    sink: &dyn EventSink,
    commands: &[MenuCommand],
    id: u32,
) -> Result<Option<MenuCommand>, WinError> {
    let Some(command) = command_for(commands, id) else {
        return Ok(None);
    };
    sink.post(Event::Menu(command))?;
    Ok(Some(command))
}

/// The access key: the character after the first single `&`; `&&` is a literal `&` (the Win32 rule).
fn access_key(text: &str) -> Option<char> {
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.next() {
                Some('&') => {}
                other => return other,
            }
        }
    }
    None
}

/// The `WM_MENUCHAR` answer for `key` (a UTF-16 unit) in a menu of `items`: `MNC_EXECUTE` with the place of the first
/// enabled item whose access key it is (for a submenu that opens it), else `MNC_IGNORE`. A status line has no key.
fn menu_char<'a>(key: u16, items: impl IntoIterator<Item = &'a MenuItem>) -> isize {
    let ignore = (MNC_IGNORE as isize) << 16;
    let Some(key) = char::from_u32(u32::from(key)) else {
        return ignore;
    };
    for (pos, item) in items.into_iter().enumerate() {
        let text = match item {
            MenuItem::Action {
                text,
                enabled: true,
                ..
            }
            | MenuItem::Submenu { text, .. } => text,
            _ => continue,
        };
        if access_key(text).is_some_and(|k| k.to_lowercase().eq(key.to_lowercase())) {
            return ((MNC_EXECUTE as isize) << 16) | pos as isize;
        }
    }
    ignore
}

/// Left or right of the anchor from `SM_MENUDROPALIGNMENT` (Microsoft's notification icon sample does the same);
/// above it when the click came from the lower half of its monitor (a bottom taskbar), else below it.
fn alignment(drop_right: bool, anchor: Anchor, monitor: RECT) -> TRACK_POPUP_MENU_FLAGS {
    let horizontal = if drop_right {
        TPM_RIGHTALIGN
    } else {
        TPM_LEFTALIGN
    };
    let middle = monitor.top + (monitor.bottom - monitor.top) / 2;
    let vertical = if anchor.y >= middle {
        TPM_BOTTOMALIGN
    } else {
        TPM_TOPALIGN
    };
    horizontal | vertical
}

/// The DPI and the rect of the anchor's monitor. The DPI is the monitor's, never the system DPI fixed at sign-in
/// (research, "`LoadIconMetric` follows the system DPI"); that one stands in only when the read fails.
fn monitor_at(anchor: Anchor) -> (u32, RECT) {
    let point = POINT {
        x: anchor.x,
        y: anchor.y,
    };
    // SAFETY: plain call; "nearest" always gives a monitor.
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    let (mut x, mut y) = (0u32, 0u32);
    // SAFETY: both out pointers are live for the call.
    let dpi = match unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) } {
        Ok(()) if x > 0 => x,
        // SAFETY: plain call.
        _ => unsafe { GetDpiForSystem() },
    };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` is live with `cbSize` set.
    let rect = if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        info.rcMonitor
    } else {
        // An empty rect at the origin opens the menu above the click, as from a bottom taskbar.
        RECT::default()
    };
    (dpi, rect)
}

/// `PreferredAppMode` (uxtheme, undocumented): the 1903 and later values from `ysc3839/win32-darkmode` `DarkMode.h`,
/// which Phase 0 matched on build 26300 (research, "Dark menus").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
enum AppMode {
    Default = 0,
    ForceDark = 2,
    ForceLight = 3,
}

/// The frame follows the Windows mode, as the items do. In high contrast it is left to Windows, which draws contrast
/// colours there and turns dark mode off for apps (what `ForceDark` does under a contrast theme is not verified).
fn mode_for(theme: &Theme) -> AppMode {
    if theme.contrast.is_some() {
        AppMode::Default
    } else if theme.light {
        AppMode::ForceLight
    } else {
        AppMode::ForceDark
    }
}

type SetPreferredAppMode = unsafe extern "system" fn(i32) -> i32;
type FlushMenuThemes = unsafe extern "system" fn();

/// uxtheme's `SetPreferredAppMode` (ordinal 135) and `FlushMenuThemes` (136), found at run time because they are not
/// exported by name; either may be gone on a later build, and then the frame keeps Windows' default.
#[derive(Clone, Copy, Debug, Default)]
struct UxTheme {
    set_mode: Option<SetPreferredAppMode>,
    flush: Option<FlushMenuThemes>,
}

impl UxTheme {
    fn load() -> Self {
        // SAFETY: a system DLL, searched in System32 only. It is never freed, so the pointers below stay valid.
        let Ok(dll) =
            (unsafe { LoadLibraryExW(w!("uxtheme.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) })
        else {
            return Self::default();
        };
        // An ordinal goes in the low word of the name pointer (`MAKEINTRESOURCEA`).
        // SAFETY: the module is loaded; an ordinal it lacks gives `None`.
        let by_ordinal =
            |n: usize| unsafe { GetProcAddress(dll, PCSTR(std::ptr::without_provenance(n))) };
        Self {
            // SAFETY: on Windows 11 ordinal 135 is `SetPreferredAppMode(PreferredAppMode) -> PreferredAppMode`
            // (Phase 0 saw it return the previous mode).
            set_mode: by_ordinal(135).map(|f| unsafe {
                std::mem::transmute::<unsafe extern "system" fn() -> isize, SetPreferredAppMode>(f)
            }),
            // SAFETY: on Windows 11 ordinal 136 is `FlushMenuThemes()`, with no arguments and no result.
            flush: by_ordinal(136).map(|f| unsafe {
                std::mem::transmute::<unsafe extern "system" fn() -> isize, FlushMenuThemes>(f)
            }),
        }
    }

    /// Sets the mode of this process's next menu frame; false when uxtheme lacks either call.
    fn set(&self, mode: AppMode) -> bool {
        let (Some(set_mode), Some(flush)) = (self.set_mode, self.flush) else {
            return false;
        };
        // SAFETY: the functions `load` found, called with their own signatures.
        unsafe {
            set_mode(mode as i32);
            flush();
        }
        true
    }
}

fn set_attribute<T>(
    window: HWND,
    attribute: DWMWINDOWATTRIBUTE,
    value: &T,
) -> Result<(), WinError> {
    // SAFETY: `value` is a live value of the type the attribute takes, for the call.
    unsafe {
        DwmSetWindowAttribute(
            window,
            attribute,
            std::ptr::from_ref(value).cast::<c_void>(),
            size_of::<T>() as u32,
        )
    }
    .map_err(|e| win_error("DwmSetWindowAttribute", &e))
}

fn last_error(call: &'static str) -> WinError {
    // SAFETY: no arguments.
    let code = unsafe { GetLastError() }.0;
    WinError { call, code }
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
    use crate::sink::FakeEventSink;
    use crate::tray::theme::{Contrast, Rgb};
    use wakeshot_core::text;
    use wakeshot_core::view::KeepAwake;
    use windows::Win32::Foundation::ERROR_INVALID_DATA;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
        DeleteDC, GdiFlush, HBITMAP, HDC, HGDIOBJ, SelectObject,
    };
    use windows::Win32::UI::Controls::{ODA_DRAWENTIRE, ODS_FLAGS};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetMenuInfo, GetMenuItemCount, GetMenuItemInfoW, IsWindow, IsWindowVisible, SendMessageW,
    };

    const DARK: Theme = Theme {
        light: false,
        contrast: None,
    };

    fn action(command: MenuCommand, checked: bool, enabled: bool) -> MenuItem {
        MenuItem::Action {
            text: text::action(command),
            command,
            checked,
            enabled,
        }
    }

    const LENGTHS: [KeepAwake; 6] = [
        KeepAwake::Off,
        KeepAwake::Minutes(30),
        KeepAwake::Minutes(60),
        KeepAwake::Minutes(120),
        KeepAwake::Minutes(240),
        KeepAwake::UntilOff,
    ];

    /// The menu as `present` builds it, with Keep awake off: Off is checked and Keep screen on is disabled.
    fn product_menu() -> Vec<MenuItem> {
        let mut keep_awake: Vec<MenuItem> = LENGTHS
            .into_iter()
            .map(|k| action(MenuCommand::KeepAwake(k), k == KeepAwake::Off, true))
            .collect();
        keep_awake.push(MenuItem::Separator);
        keep_awake.push(action(MenuCommand::KeepScreenOn(true), false, false));
        vec![
            MenuItem::Label("Holding awake: R&&D build".to_owned()),
            MenuItem::Separator,
            action(MenuCommand::Automatic(false), true, true),
            MenuItem::Submenu {
                text: text::keep_awake_menu(),
                items: keep_awake,
            },
            MenuItem::Separator,
            action(MenuCommand::Restore, false, true),
            action(MenuCommand::OpenLog, false, true),
            action(MenuCommand::Quit, false, true),
        ]
    }

    fn menu_item(menu: HMENU, pos: u32) -> MENUITEMINFOW {
        let mut mii = MENUITEMINFOW {
            cbSize: size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_FTYPE | MIIM_STATE | MIIM_ID | MIIM_DATA | MIIM_SUBMENU,
            ..Default::default()
        };
        // SAFETY: `mii` is live with `cbSize` set.
        unsafe { GetMenuItemInfoW(menu, pos, true, &mut mii) }.expect("GetMenuItemInfoW");
        mii
    }

    fn count(menu: HMENU) -> i32 {
        // SAFETY: plain call.
        unsafe { GetMenuItemCount(Some(menu)) }
    }

    #[test]
    fn ids_round_trip_every_command() {
        // Every command the core can send, a length the menu does not offer today included.
        let mut every = vec![
            MenuCommand::Automatic(true),
            MenuCommand::Automatic(false),
            MenuCommand::KeepAwake(KeepAwake::Minutes(90)),
            MenuCommand::KeepScreenOn(true),
            MenuCommand::KeepScreenOn(false),
            MenuCommand::Restore,
            MenuCommand::OpenLog,
            MenuCommand::Quit,
        ];
        every.extend(LENGTHS.map(MenuCommand::KeepAwake));
        // Half of them in a submenu between the others, so the walk order shows.
        let (outer, inner) = every.split_at(4);
        let mut items: Vec<MenuItem> = outer[..2].iter().map(|&c| action(c, false, true)).collect();
        items.push(MenuItem::Submenu {
            text: text::keep_awake_menu(),
            items: inner.iter().map(|&c| action(c, false, true)).collect(),
        });
        items.extend(outer[2..].iter().map(|&c| action(c, false, true)));
        let in_order: Vec<MenuCommand> = outer[..2]
            .iter()
            .chain(inner)
            .chain(&outer[2..])
            .copied()
            .collect();

        let commands = command_ids(&items);
        assert_eq!(commands, in_order);
        for (i, &command) in in_order.iter().enumerate() {
            let id = u32::try_from(i + 1).unwrap();
            assert_eq!(command_for(&commands, id), Some(command), "id {id}");
        }
        assert_eq!(command_for(&commands, 0), None, "0 is nothing picked");
        let past = u32::try_from(commands.len() + 1).unwrap();
        assert_eq!(command_for(&commands, past), None);
        assert_eq!(command_for(&commands, u32::MAX), None);
    }

    #[test]
    fn labels_get_no_id() {
        let items = vec![
            MenuItem::Label("Idle".to_owned()),
            MenuItem::Separator,
            MenuItem::Submenu {
                text: text::keep_awake_menu(),
                items: vec![
                    MenuItem::Label("Inner".to_owned()),
                    action(MenuCommand::KeepAwake(KeepAwake::Off), true, true),
                ],
            },
            action(MenuCommand::Quit, false, true),
        ];
        assert_eq!(
            command_ids(&items),
            [MenuCommand::KeepAwake(KeepAwake::Off), MenuCommand::Quit]
        );
        let open = Open::new(&items, &DARK, 96).expect("open");
        assert_eq!(menu_item(open.root, 0).wID, 0, "label");
        assert_eq!(menu_item(open.root, 1).wID, 0, "separator");
        let submenu = menu_item(open.root, 2);
        assert_eq!(submenu.wID, 0, "submenu");
        assert_eq!(menu_item(submenu.hSubMenu, 0).wID, 0, "inner label");
        assert_eq!(menu_item(submenu.hSubMenu, 1).wID, 1);
        assert_eq!(menu_item(open.root, 3).wID, 2);
    }

    #[test]
    fn access_key_found_and_double_ampersand_ignored() {
        assert_eq!(access_key("&Keep awake"), Some('K'));
        assert_eq!(access_key("Keep &screen on"), Some('s'));
        assert_eq!(access_key("Save && &Quit"), Some('Q'));
        assert_eq!(access_key("&&&x"), Some('x'));
        assert_eq!(access_key("R&&D build"), None);
        assert_eq!(access_key("Automatic"), None);
        assert_eq!(access_key("ends in &"), None);
        assert_eq!(access_key("&\u{561}\u{580}"), Some('\u{561}'));
        // Every product command has one.
        for command in command_ids(&product_menu()) {
            assert!(access_key(&text::action(command)).is_some(), "{command:?}");
        }
        assert!(access_key(&text::keep_awake_menu()).is_some());
    }

    #[test]
    fn msaa_text_is_the_menu_text() {
        let layout = Layout::new(96, Columns::default());
        for text in ["&Keep awake", "Holding awake: R&&D build", "\u{54a}\u{561}"] {
            let data = ItemData::new(&MenuItem::Label(text.to_owned()), layout);
            assert_eq!(data.msaa.dwMSAASignature, MSAA_MENU_SIG as u32);
            let units: Vec<u16> = text.encode_utf16().collect();
            assert_eq!(data.msaa.cchWText as usize, units.len(), "{text}");
            // SAFETY: `pszWText` points at `cchWText` units and an end mark, owned by `data`.
            let read = unsafe {
                std::slice::from_raw_parts(data.msaa.pszWText.0, units.len() + 1).to_vec()
            };
            assert_eq!(read[..units.len()], units[..], "{text}");
            assert_eq!(read[units.len()], 0, "end mark");
        }
        // A screen reader finds the MSAA data at the start of the item data.
        assert_eq!(std::mem::offset_of!(ItemData, msaa), 0);
        let separator = ItemData::new(&MenuItem::Separator, layout);
        assert_eq!(separator.msaa.cchWText, 0);
    }

    #[test]
    fn menu_char_picks_the_item_or_ignores() {
        let items = product_menu();
        let execute = |pos: isize| ((MNC_EXECUTE as isize) << 16) | pos;
        let ignore = (MNC_IGNORE as isize) << 16;
        let key = |c: char| c as u16;
        assert_eq!(menu_char(key('k'), &items), execute(3), "opens the submenu");
        assert_eq!(menu_char(key('K'), &items), execute(3), "either case");
        assert_eq!(menu_char(key('a'), &items), execute(2));
        assert_eq!(menu_char(key('q'), &items), execute(7));
        assert_eq!(menu_char(key('x'), &items), ignore);
        // The status line "R&&D" has no key, so 'd' is nobody's.
        assert_eq!(menu_char(key('d'), &items), ignore);
        // A lone surrogate is no character.
        assert_eq!(menu_char(0xD800, &items), ignore);
        let MenuItem::Submenu { items: inner, .. } = &items[3] else {
            panic!("submenu");
        };
        assert_eq!(menu_char(key('u'), inner), execute(5));
        assert_eq!(menu_char(key('o'), inner), execute(0));
        // Keep screen on is disabled here.
        assert_eq!(menu_char(key('s'), inner), ignore);
    }

    #[test]
    fn builds_the_hmenu_from_the_model() {
        let items = product_menu();
        let open = Open::new(&items, &DARK, 120).expect("open");
        assert_eq!(open.commands, command_ids(&items));
        assert_eq!(count(open.root), 8);
        assert_eq!(open.menus.len(), 2, "the root and one submenu");

        let check = |menu: HMENU, model: &[MenuItem]| {
            for (pos, item) in model.iter().enumerate() {
                let mii = menu_item(menu, pos as u32);
                assert!(mii.fType.0 & MFT_OWNERDRAW.0 != 0, "{item:?}");
                let data = open.data(mii.dwItemData).expect("our item data");
                assert_eq!(&data.item, item);
                let separator = mii.fType.0 & MFT_SEPARATOR.0 != 0;
                let grayed = mii.fState.0 & MFS_GRAYED.0 != 0;
                let checked = mii.fState.0 & MFS_CHECKED.0 != 0;
                let radio = mii.fType.0 & MFT_RADIOCHECK.0 != 0;
                match item {
                    MenuItem::Separator => assert!(separator && mii.wID == 0),
                    MenuItem::Label(_) => assert!(grayed && !separator && mii.wID == 0),
                    MenuItem::Action {
                        command,
                        checked: c,
                        enabled,
                        ..
                    } => {
                        assert_eq!(command_for(&open.commands, mii.wID), Some(*command));
                        assert_eq!(checked, *c, "{item:?}");
                        assert_eq!(grayed, !*enabled, "{item:?}");
                        assert_eq!(radio, matches!(command, MenuCommand::KeepAwake(_)));
                        assert!(mii.hSubMenu.is_invalid());
                    }
                    MenuItem::Submenu { items: inner, .. } => {
                        assert_eq!(mii.wID, 0);
                        assert_eq!(count(mii.hSubMenu), inner.len() as i32);
                    }
                }
            }
        };
        check(open.root, &items);
        let MenuItem::Submenu { items: inner, .. } = &items[3] else {
            panic!("submenu");
        };
        let submenu = menu_item(open.root, 3).hSubMenu;
        check(submenu, inner);

        // Each menu keeps its own columns: the main menu has marks and icons, the lengths only marks.
        let layout =
            |menu: HMENU, pos: u32| open.data(menu_item(menu, pos).dwItemData).unwrap().layout;
        assert!(layout(open.root, 2).columns.icons);
        assert!(!layout(submenu, 0).columns.icons);
        assert!(layout(submenu, 0).columns.marks);
        assert_eq!(layout(submenu, 0).dpi, 120);

        // The gutter takes the palette's fill, in the submenu too.
        for menu in [open.root, submenu] {
            let mut info = MENUINFO {
                cbSize: size_of::<MENUINFO>() as u32,
                fMask: MIM_BACKGROUND,
                ..Default::default()
            };
            // SAFETY: `info` is live with `cbSize` set.
            unsafe { GetMenuInfo(menu, &mut info) }.expect("GetMenuInfo");
            assert_eq!(info.hbrBack, open.brush);
        }
        assert_eq!(open.palette, Palette::for_theme(&DARK));
    }

    #[test]
    fn pick_posts_menu_event() {
        let commands = command_ids(&product_menu());
        let quit = u32::try_from(commands.len()).unwrap();
        let sink = FakeEventSink::default();
        assert_eq!(
            post_pick(&sink, &commands, quit),
            Ok(Some(MenuCommand::Quit))
        );
        assert_eq!(sink.events(), [Event::Menu(MenuCommand::Quit)]);
        assert_eq!(post_pick(&sink, &commands, 0), Ok(None), "Esc");
        assert_eq!(post_pick(&sink, &commands, quit + 1), Ok(None));
        assert_eq!(sink.events().len(), 1, "nothing more posted");

        let error = WinError {
            call: "PostMessageW",
            code: ERROR_INVALID_DATA.0,
        };
        let failing = FakeEventSink {
            fail_with: Some(error),
            ..Default::default()
        };
        assert_eq!(post_pick(&failing, &commands, 1), Err(error));
    }

    #[test]
    fn alignment_follows_the_setting_and_the_anchor() {
        let screen = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1200,
        };
        let taskbar = Anchor { x: 1800, y: 1180 };
        assert_eq!(
            alignment(false, taskbar, screen),
            TPM_LEFTALIGN | TPM_BOTTOMALIGN
        );
        assert_eq!(
            alignment(true, taskbar, screen),
            TPM_RIGHTALIGN | TPM_BOTTOMALIGN
        );
        let top = Anchor { x: 1800, y: 10 };
        assert_eq!(alignment(false, top, screen), TPM_LEFTALIGN | TPM_TOPALIGN);
        // A screen above the primary has negative coordinates.
        let above = RECT {
            left: 0,
            top: -1440,
            right: 2560,
            bottom: 0,
        };
        assert_eq!(
            alignment(false, Anchor { x: 100, y: -20 }, above),
            TPM_LEFTALIGN | TPM_BOTTOMALIGN
        );
        assert_eq!(
            alignment(false, Anchor { x: 100, y: -1400 }, above),
            TPM_LEFTALIGN | TPM_TOPALIGN
        );
    }

    #[test]
    fn frame_mode_follows_the_theme() {
        assert_eq!(mode_for(&DARK), AppMode::ForceDark);
        let light = Theme {
            light: true,
            contrast: None,
        };
        assert_eq!(mode_for(&light), AppMode::ForceLight);
        let black = Rgb::new(0, 0, 0);
        let contrast = Theme {
            light: true,
            contrast: Some(Contrast {
                window: black,
                window_text: black,
                highlight: black,
                highlight_text: black,
                gray_text: black,
                button_face: black,
                button_text: black,
                hot_light: black,
            }),
        };
        assert_eq!(mode_for(&contrast), AppMode::Default);
        // The ordinals resolve on this PC (research, "Dark menus").
        let ux = UxTheme::load();
        assert!(ux.set_mode.is_some() && ux.flush.is_some());
    }

    /// A 32-bit top-down DIB in a memory DC, as a menu window's DC stands in for it.
    struct Surface {
        dc: HDC,
        bitmap: HBITMAP,
        old: HGDIOBJ,
        bits: *const u32,
        w: i32,
        h: i32,
    }

    impl Surface {
        fn new(w: i32, h: i32) -> Self {
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
            // SAFETY: `info` describes a 32-bit DIB; the DC and the bitmap are freed on drop.
            unsafe {
                let dc = CreateCompatibleDC(None);
                let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
                    .expect("dib");
                let old = SelectObject(dc, bitmap.into());
                Self {
                    dc,
                    bitmap,
                    old,
                    bits: bits.cast(),
                    w,
                    h,
                }
            }
        }

        fn at(&self, x: i32, y: i32) -> u32 {
            assert!(x < self.w && y < self.h);
            // SAFETY: GDI is flushed, and the pixel is inside the bitmap.
            unsafe {
                let _ = GdiFlush();
                *self.bits.add((y * self.w + x) as usize)
            }
        }
    }

    impl Drop for Surface {
        fn drop(&mut self) {
            // SAFETY: made in `new`; the old bitmap goes back before the DIB is deleted.
            unsafe {
                SelectObject(self.dc, self.old);
                let _ = DeleteObject(self.bitmap.into());
                let _ = DeleteDC(self.dc);
            }
        }
    }

    #[test]
    fn owner_window_measures_draws_and_maps_keys() {
        let sink = Arc::new(FakeEventSink::default());
        let mut menu = OwnerDrawnMenu::new(sink.clone()).expect("owner window");
        let hwnd = menu.hwnd;
        // SAFETY: plain calls on a window of this thread.
        assert!(unsafe { IsWindow(Some(hwnd)) }.as_bool());
        assert!(!unsafe { IsWindowVisible(hwnd) }.as_bool(), "never shown");
        let notes = Rc::new(RefCell::new(Vec::new()));
        let seen = notes.clone();
        menu.on_note(Box::new(move |note| seen.borrow_mut().push(note.clone())));

        let items = product_menu();
        let open = Open::new(&items, &DARK, 96).expect("open");
        let root = open.root;
        let quit = menu_item(root, 7).dwItemData;
        let background = Palette::for_theme(&DARK).background;
        let layout = open.data(quit).unwrap().layout;
        *menu.shared.open.borrow_mut() = Some(open);
        let send = |msg: u32, wparam: usize, lparam: isize| {
            // SAFETY: a window of this thread; each message's pointer is live for the call.
            unsafe { SendMessageW(hwnd, msg, Some(WPARAM(wparam)), Some(LPARAM(lparam))) }.0
        };

        let mut mis = MEASUREITEMSTRUCT {
            CtlType: ODT_MENU,
            itemData: quit,
            ..Default::default()
        };
        assert_eq!(send(WM_MEASUREITEM, 0, &raw mut mis as isize), 1);
        assert_eq!(mis.itemHeight, 33, "a row at 96 dpi");
        assert!(mis.itemWidth > 0);
        // Item data that is not ours is never read.
        let foreign = 7usize;
        let mut other = MEASUREITEMSTRUCT {
            CtlType: ODT_MENU,
            itemData: std::ptr::from_ref(&foreign) as usize,
            ..Default::default()
        };
        assert_eq!(send(WM_MEASUREITEM, 0, &raw mut other as isize), 0);
        assert_eq!(other.itemHeight, 0);

        let surface = Surface::new(200, layout.row);
        let dis = DRAWITEMSTRUCT {
            CtlType: ODT_MENU,
            itemAction: ODA_DRAWENTIRE,
            itemState: ODS_FLAGS(0),
            hDC: surface.dc,
            rcItem: RECT {
                left: 0,
                top: 0,
                right: 200,
                bottom: layout.row,
            },
            itemData: quit,
            ..Default::default()
        };
        assert_eq!(send(WM_DRAWITEM, 0, &raw const dis as isize), 1);
        let fill =
            u32::from(background.r) << 16 | u32::from(background.g) << 8 | u32::from(background.b);
        assert_eq!(
            surface.at(1, 1) & 0x00ff_ffff,
            fill,
            "painted in the palette"
        );

        let q = 'q' as usize;
        assert_eq!(
            send(WM_MENUCHAR, q, root.0 as isize),
            ((MNC_EXECUTE as isize) << 16) | 7
        );

        // The memory DC has no window, so nothing was styled; a window that is not a menu fails DWM's call or not,
        // but it is styled only once.
        send(WM_ENTERIDLE, MSGF_MENU as usize, hwnd.0 as isize);
        send(WM_ENTERIDLE, MSGF_MENU as usize, hwnd.0 as isize);
        let styled = notes
            .borrow()
            .iter()
            .filter(|n| {
                matches!(
                    n,
                    Note::Styled {
                        at: StyledAt::Idle,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(styled, 1);

        // WM_CLOSE leaves the window to its owner.
        send(WM_CLOSE, 0, 0);
        // SAFETY: plain call.
        assert!(unsafe { IsWindow(Some(hwnd)) }.as_bool());
        assert!(sink.events().is_empty(), "no pick without a menu loop");
        drop(menu);
        // SAFETY: plain call.
        assert!(
            !unsafe { IsWindow(Some(hwnd)) }.as_bool(),
            "drop destroys it"
        );
    }
}
