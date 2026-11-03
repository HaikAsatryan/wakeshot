//! `TrayIcon`: its own hidden window, the notify icon, clicks, re-add, theme and display changes.
//!
//! Owner: `p5-tray-icon`.
//!
//! Every piece of state sits in one `Shared` behind `Cell` and `RefCell`, reached from the window procedure through an
//! `Rc`, and no borrow is held across a call that can run other code: `Shell_NotifyIconW` (sent messages such as a
//! theme broadcast are dispatched while it waits on Explorer), `MenuView::show` (a modal loop that dispatches this
//! window's messages and the loop's events, which call `TrayIcon::show`) and the hooks. While the menu is open no shell
//! call is made: a new view, theme or size is kept and drawn when the menu closes.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;
use std::time::Duration;

use wakeshot_core::text;
use wakeshot_core::view::{IconState, TrayView};
use windows::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject, HBITMAP,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::{
    NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NIM_SETVERSION, NIN_BALLOONUSERCLICK, NIN_SELECT, NINF_KEY, NOTIFY_ICON_MESSAGE,
    NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    ChangeWindowMessageFilterEx, CreateIconIndirect, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyWindow, FindWindowW, GWLP_USERDATA, GetWindowLongPtrW, HICON, ICONINFO, KillTimer,
    MSGFLT_ALLOW, RegisterClassExW, RegisterWindowMessageW, SM_CXSMICON, SPI_SETHIGHCONTRAST,
    SetTimer, SetWindowLongPtrW, WM_APP, WM_CLOSE, WM_CONTEXTMENU, WM_DISPLAYCHANGE,
    WM_SETTINGCHANGE, WM_SYSCOLORCHANGE, WM_TIMER, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};
use windows::core::{GUID, PCWSTR, w};

use super::art::{self, Mask};
use super::theme::{self, Theme, ThemeWatch};
use super::{Anchor, IconIdentity, MenuView};
use crate::WinError;

/// The icon's callback message. The window is this module's own, so any `WM_APP` value is free.
const WM_TRAY: u32 = WM_APP + 0x10;

/// `NIF_GUID` identifies the icon; the id only comes back in the callback's high word, and 0 is right whether Windows
/// echoes the `uID` given or sends 0 for a GUID icon (which one: Part B logs it).
const ICON_UID: u32 = 0;

/// `windows` 0.62.2 has no `NIN_KEYSELECT`.
const NIN_KEYSELECT: u32 = NIN_SELECT | NINF_KEY;

const TIMER_RETRY: usize = 1;
const TIMER_SETTLE: usize = 2;

/// "ImmersiveColorSet" comes twice per switch, the first time with stale values (research, "Live theme switch through
/// Settings"), so the theme is read at once and again after this.
const SETTLE: Duration = Duration::from_secs(1);

/// `szTip` holds 128 UTF-16 units with the end mark.
const TIP_MAX_UTF16: usize = 127;

const CLASS: PCWSTR = w!("Wakeshot.TrayIcon");

/// Which notify icon: each kind has its own fixed GUID, so a dry-run tray and a demo run beside the real one, and
/// Windows keeps each one's pinned state apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconKind {
    Product,
    DryRun,
    Demo,
}

impl IconKind {
    /// Made once for `p5-tray-icon` (2026-11-04). Windows keys the icon's settings entry (pinned or not) to it, so a
    /// GUID never changes once shipped.
    pub const fn guid(self) -> u128 {
        match self {
            IconKind::Product => 0x7db9403e_d4b9_4165_a99e_a69b8ea56744,
            IconKind::DryRun => 0xc0d07dcd_dfd4_4a9d_b139_ad1e2ce9f987,
            IconKind::Demo => 0x1e8c66d4_0fa3_4a54_95a1_61bb89b171e9,
        }
    }
}

/// What a callback message asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// Left click, keyboard select and right click all open the same menu (track note 3.4).
    Open {
        by: OpenBy,
        anchor: Anchor,
    },
    BalloonClick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenBy {
    Click,
    Key,
    ContextMenu,
}

/// What the icon did, for the log and the demos.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    Added {
        attempt: u32,
    },
    AddFailed {
        attempt: u32,
        retry_in: Duration,
    },
    Redrawn {
        icon: IconState,
        size: u32,
        colour: [u8; 3],
    },
    ModifyFailed,
    TaskbarCreated,
    ThemeChanged(Theme),
    SizeChanged(u32),
}

/// Called with each `Note`.
pub type NoteHook = Box<dyn FnMut(&Note)>;

/// The notify icon, on the loop thread only (`Rc` keeps it off other threads).
pub struct TrayIcon {
    hwnd: HWND,
    shared: Rc<Shared>,
}

impl TrayIcon {
    /// Makes the hidden window and reads the theme and the taskbar's size. The icon is added at the first `show`.
    pub fn new(kind: IconKind, menu: Box<dyn MenuView>) -> Result<Self, WinError> {
        Self::with_shell(kind, menu, Box::new(RealShell))
    }

    fn with_shell(
        kind: IconKind,
        menu: Box<dyn MenuView>,
        shell: Box<dyn Shell>,
    ) -> Result<Self, WinError> {
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
            // A second icon in this process (tests, a demo) finds the class registered.
            if error != ERROR_CLASS_ALREADY_EXISTS {
                return Err(WinError {
                    call: "RegisterClassExW",
                    code: error.0,
                });
            }
        }
        // Top-level, not message-only: only top-level windows get `TaskbarCreated` (research, "Explorer restart").
        // SAFETY: the class is registered above; the window is never shown and is destroyed in `Drop`, on this thread.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                CLASS,
                w!("Wakeshot tray icon"),
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
        // SAFETY: a static NUL-terminated name.
        let taskbar_created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        if taskbar_created != 0 {
            // An elevated tray would not get Explorer's broadcast otherwise. A failure only matters when elevated, and
            // the tray never is by design, so it is not an error.
            // SAFETY: this thread's own window.
            let _ =
                unsafe { ChangeWindowMessageFilterEx(hwnd, taskbar_created, MSGFLT_ALLOW, None) };
        }
        // A failed read draws for a dark taskbar, as the menu does; the next good read corrects it.
        let first = theme::read().unwrap_or(Theme {
            light: false,
            contrast: None,
        });
        let shared = Rc::new(Shared {
            hwnd,
            kind,
            taskbar_created,
            shell,
            menu: RefCell::new(menu),
            state: RefCell::new(State {
                view: None,
                tip: String::new(),
                theme: ThemeWatch::new(first),
                forced_theme: None,
                pattern: false,
                size: glyph_size(taskbar_dpi()),
                tried: false,
                added: false,
                failures: 0,
                drawn: None,
                shown_tip: None,
                icon: None,
            }),
            menu_open: Cell::new(false),
            syncing: Cell::new(false),
            dirty: Cell::new(false),
            closed: Cell::new(false),
            balloon_hook: RefCell::new(None),
            note_hook: RefCell::new(None),
        });
        // SAFETY: the window made above; `Drop` clears the slot before this `Rc` can go.
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, Rc::as_ptr(&shared) as isize) };
        Ok(TrayIcon { hwnd, shared })
    }

    /// Shows `view`: the first call adds the icon; later calls change it only when the state, the theme or the
    /// tooltip changed. `view.menu` opens on the next click. While the menu is open the view waits until it closes.
    pub fn show(&mut self, view: &TrayView) {
        let tip = match self.shared.kind {
            IconKind::DryRun => text::dry_run(&view.tooltip),
            IconKind::Product | IconKind::Demo => view.tooltip.clone(),
        };
        {
            let mut st = self.shared.state.borrow_mut();
            st.view = Some(view.clone());
            st.tip = cut_tooltip(&tip).to_owned();
        }
        self.shared.sync();
    }

    /// Called on a balloon click (`NIN_BALLOONUSERCLICK`); `BalloonNotifier` keeps what the click opens.
    pub fn on_balloon_click(&mut self, hook: Box<dyn FnMut()>) {
        *self.shared.balloon_hook.borrow_mut() = Some(hook);
    }

    /// Called with each thing the icon did (added, add failed, redrawn, theme or size changed).
    pub fn on_note(&mut self, hook: NoteHook) {
        *self.shared.note_hook.borrow_mut() = Some(hook);
    }

    /// For `BalloonNotifier`, which changes this icon from the IO thread.
    pub fn identity(&self) -> IconIdentity {
        IconIdentity {
            hwnd: self.hwnd.0 as isize,
            guid: self.shared.kind.guid(),
        }
    }

    /// Demos only (`ux_icon --contrast`): draw with this theme instead of the live one; `None` goes back to it.
    pub fn force_theme(&mut self, theme: Option<Theme>) {
        self.shared.state.borrow_mut().forced_theme = theme;
        self.shared.sync();
    }

    /// Demos only (`ux_icon --pattern`): a square with a one-pixel border and a one-pixel checker inside, in place of
    /// the glyph, so a picture shows whether the taskbar draws it 1:1 or scales it.
    pub fn show_pattern(&mut self, on: bool) {
        self.shared.state.borrow_mut().pattern = on;
        self.shared.sync();
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        let shared = &*self.shared;
        shared.closed.set(true);
        // SAFETY: this thread's own window and timers; a timer that was not set fails harmlessly.
        unsafe {
            let _ = KillTimer(Some(self.hwnd), TIMER_RETRY);
            let _ = KillTimer(Some(self.hwnd), TIMER_SETTLE);
        }
        // Also when no add worked: an add that timed out may still have landed.
        if shared.state.borrow().tried {
            shared.shell.notify(NIM_DELETE, &shared.data());
        }
        // SAFETY: this thread's own window. The slot is cleared first, so the messages `DestroyWindow` sends never
        // see the state. A call still running (an open menu) holds its own `Rc`.
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

/// `Shell_NotifyIconW`, behind a seam so no test puts an icon on the taskbar.
trait Shell {
    fn notify(&self, op: NOTIFY_ICON_MESSAGE, data: &NOTIFYICONDATAW) -> bool;
}

struct RealShell;

impl Shell for RealShell {
    fn notify(&self, op: NOTIFY_ICON_MESSAGE, data: &NOTIFYICONDATAW) -> bool {
        // SAFETY: `data` is a live, fully initialised NOTIFYICONDATAW with `cbSize` set.
        unsafe { Shell_NotifyIconW(op, data) }.as_bool()
    }
}

struct Shared {
    hwnd: HWND,
    kind: IconKind,
    /// 0 when the registration failed: then it never matches, since message 0 is `WM_NULL` and is checked first.
    taskbar_created: u32,
    shell: Box<dyn Shell>,
    menu: RefCell<Box<dyn MenuView>>,
    state: RefCell<State>,
    menu_open: Cell<bool>,
    syncing: Cell<bool>,
    /// Something changed during a sync or while the menu was open: sync again.
    dirty: Cell<bool>,
    closed: Cell<bool>,
    balloon_hook: RefCell<Option<Box<dyn FnMut()>>>,
    note_hook: RefCell<Option<NoteHook>>,
}

struct State {
    view: Option<TrayView>,
    /// The view's tooltip as shown: dry-run mark added, cut to fit.
    tip: String,
    theme: ThemeWatch,
    forced_theme: Option<Theme>,
    pattern: bool,
    size: u32,
    /// An add was sent at least once, so `Drop` deletes.
    tried: bool,
    added: bool,
    /// Failed adds in a row.
    failures: u32,
    /// What the shell shows now; `None` before the first add and after `TaskbarCreated`.
    drawn: Option<Drawn>,
    shown_tip: Option<String>,
    /// The handle the shell got last. The shell keeps its own copy, but it is freed only after the next call that
    /// replaced it.
    icon: Option<Icon>,
}

impl State {
    fn want(&self) -> Option<Drawn> {
        let view = self.view.as_ref()?;
        let theme = self.forced_theme.unwrap_or(self.theme.current());
        Some(Drawn {
            icon: view.icon,
            colour: glyph_colour(&theme),
            size: self.size,
            pattern: self.pattern,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Drawn {
    icon: IconState,
    colour: [u8; 3],
    size: u32,
    pattern: bool,
}

enum Plan {
    Add(Drawn, String),
    Modify {
        want: Drawn,
        redraw: bool,
        tip: String,
    },
}

impl Shared {
    fn data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: ICON_UID,
            uFlags: NIF_GUID,
            guidItem: GUID::from_u128(self.kind.guid()),
            ..Default::default()
        }
    }

    /// Brings the shell in line with the state. Runs again when something changed while it ran, and waits while the
    /// menu is open.
    fn sync(&self) {
        if self.closed.get() || self.menu_open.get() || self.syncing.get() {
            self.dirty.set(true);
            return;
        }
        self.syncing.set(true);
        loop {
            self.dirty.set(false);
            self.sync_once();
            if !self.dirty.get() || self.closed.get() {
                break;
            }
        }
        self.syncing.set(false);
    }

    fn sync_once(&self) {
        let plan = {
            let st = self.state.borrow();
            let Some(want) = st.want() else { return };
            if !st.added {
                Plan::Add(want, st.tip.clone())
            } else if st.drawn != Some(want) || st.shown_tip.as_deref() != Some(st.tip.as_str()) {
                Plan::Modify {
                    want,
                    redraw: st.drawn != Some(want),
                    tip: st.tip.clone(),
                }
            } else {
                return;
            }
        };
        match plan {
            Plan::Add(want, tip) => self.add(want, &tip),
            Plan::Modify { want, redraw, tip } => self.modify(want, redraw, &tip),
        }
    }

    fn add(&self, want: Drawn, tip: &str) {
        let icon = draw(want).ok();
        let mut data = self.data();
        data.uFlags = NIF_GUID | NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
        data.uCallbackMessage = WM_TRAY;
        data.hIcon = icon.as_ref().map_or(HICON::default(), |i| i.0);
        put_tip(&mut data, tip);
        self.state.borrow_mut().tried = true;
        let mut ok = icon.is_some() && self.shell.notify(NIM_ADD, &data);
        if icon.is_some() && !ok {
            // An icon left by a run that ended without `NIM_DELETE`, or an add that timed out but landed, may block
            // a new add of the same GUID (not verified). Wakeshot owns the GUID, so it deletes and tries once more.
            self.shell.notify(NIM_DELETE, &self.data());
            ok = self.shell.notify(NIM_ADD, &data);
        }
        if ok {
            data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            // Without version 4 the clicks come in the old form, which `decode` does not read; so an icon without it
            // counts as not added.
            ok = self.shell.notify(NIM_SETVERSION, &data);
            if !ok {
                self.shell.notify(NIM_DELETE, &self.data());
            }
        }
        let note = {
            let mut st = self.state.borrow_mut();
            if ok {
                st.added = true;
                let attempt = st.failures + 1;
                st.failures = 0;
                st.drawn = Some(want);
                st.shown_tip = Some(tip.to_owned());
                let old = std::mem::replace(&mut st.icon, icon);
                drop(st);
                drop(old);
                // SAFETY: this thread's own window; a timer that was not set fails harmlessly.
                let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_RETRY) };
                Note::Added { attempt }
            } else {
                st.failures += 1;
                let attempt = st.failures;
                drop(st);
                let retry_in = retry_delay(attempt);
                self.set_timer(TIMER_RETRY, retry_in);
                Note::AddFailed { attempt, retry_in }
            }
        };
        self.note(&note);
    }

    fn modify(&self, want: Drawn, redraw: bool, tip: &str) {
        let icon = if redraw {
            match draw(want) {
                Ok(icon) => Some(icon),
                // Out of GDI handles: keep the old picture; the next change tries again.
                Err(_) => return,
            }
        } else {
            None
        };
        let mut data = self.data();
        data.uFlags = NIF_GUID | NIF_TIP | NIF_SHOWTIP;
        if let Some(icon) = &icon {
            data.uFlags |= NIF_ICON;
            data.hIcon = icon.0;
        }
        put_tip(&mut data, tip);
        let ok = self.shell.notify(NIM_MODIFY, &data);
        if !ok {
            // The icon is gone (Explorer ended before its `TaskbarCreated`, or the shell dropped it): add it again.
            {
                let mut st = self.state.borrow_mut();
                st.added = false;
                st.drawn = None;
            }
            self.dirty.set(true);
            self.note(&Note::ModifyFailed);
            return;
        }
        {
            let mut st = self.state.borrow_mut();
            st.drawn = Some(want);
            st.shown_tip = Some(tip.to_owned());
            if icon.is_some() {
                let old = std::mem::replace(&mut st.icon, icon);
                drop(st);
                drop(old);
            }
        }
        if redraw {
            self.note(&Note::Redrawn {
                icon: want.icon,
                size: want.size,
                colour: want.colour,
            });
        }
    }

    fn handle(&self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if self.closed.get() {
            // SAFETY: the default handling, with the message as it came.
            return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
        }
        if msg != 0 && msg == self.taskbar_created {
            self.on_taskbar_created();
            return LRESULT(0);
        }
        match msg {
            WM_TRAY => {
                if let Some(input) = decode(msg, wparam, lparam) {
                    self.on_input(input);
                }
                LRESULT(0)
            }
            WM_TIMER => {
                // Both timers are one-shot.
                // SAFETY: this thread's own window.
                let _ = unsafe { KillTimer(Some(hwnd), wparam.0) };
                match wparam.0 {
                    TIMER_RETRY => self.sync(),
                    TIMER_SETTLE => self.recheck(),
                    _ => {}
                }
                LRESULT(0)
            }
            WM_SETTINGCHANGE => {
                if is_theme_setting(wparam, lparam) {
                    self.recheck_now_and_later();
                }
                LRESULT(0)
            }
            WM_SYSCOLORCHANGE | WM_DISPLAYCHANGE => {
                self.recheck_now_and_later();
                LRESULT(0)
            }
            // The loop window owns quitting (`taskkill` posts `WM_CLOSE` to every top-level window); the default would
            // destroy this window under a live icon.
            WM_CLOSE => LRESULT(0),
            // SAFETY: the default handling, with the message as it came.
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    fn on_input(&self, input: Input) {
        match input {
            Input::Open { anchor, .. } => self.open_menu(anchor),
            Input::BalloonClick => {
                let hook = self.balloon_hook.borrow_mut().take();
                if let Some(mut hook) = hook {
                    hook();
                    let mut slot = self.balloon_hook.borrow_mut();
                    if slot.is_none() {
                        *slot = Some(hook);
                    }
                }
            }
        }
    }

    fn open_menu(&self, anchor: Anchor) {
        // A click while the menu is open does nothing: the menu's modal loop dispatches it here.
        if self.menu_open.get() {
            return;
        }
        let Some(model) = self.state.borrow().view.as_ref().map(|v| v.menu.clone()) else {
            return;
        };
        self.menu_open.set(true);
        self.menu.borrow_mut().show(&model, anchor);
        self.menu_open.set(false);
        self.sync();
    }

    fn on_taskbar_created(&self) {
        {
            let mut st = self.state.borrow_mut();
            st.added = false;
            st.drawn = None;
            st.failures = 0;
            st.size = glyph_size(taskbar_dpi());
        }
        // SAFETY: this thread's own window; a timer that was not set fails harmlessly.
        let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_RETRY) };
        self.note(&Note::TaskbarCreated);
        self.sync();
    }

    /// Theme and display changes: read at once, and again when the broadcasts have settled.
    fn recheck_now_and_later(&self) {
        self.recheck();
        self.set_timer(TIMER_SETTLE, SETTLE);
    }

    fn recheck(&self) {
        let (theme, size) = {
            let mut st = self.state.borrow_mut();
            let theme = st.theme.check();
            let size = glyph_size(taskbar_dpi());
            let size = (size != st.size).then_some(size);
            if let Some(size) = size {
                st.size = size;
            }
            (theme, size)
        };
        if let Some(theme) = theme {
            self.note(&Note::ThemeChanged(theme));
        }
        if let Some(size) = size {
            self.note(&Note::SizeChanged(size));
        }
        if theme.is_some() || size.is_some() {
            self.sync();
        }
    }

    fn set_timer(&self, id: usize, after: Duration) {
        let ms = u32::try_from(after.as_millis()).unwrap_or(u32::MAX);
        // A timer that cannot be set leaves `TaskbarCreated` and the next view to retry.
        // SAFETY: this thread's own window; no timer procedure, so `WM_TIMER` comes to `handle`.
        let _ = unsafe { SetTimer(Some(self.hwnd), id, ms, None) };
    }

    fn note(&self, note: &Note) {
        let hook = self.note_hook.borrow_mut().take();
        if let Some(mut hook) = hook {
            hook(note);
            let mut slot = self.note_hook.borrow_mut();
            if slot.is_none() {
                *slot = Some(hook);
            }
        }
    }
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: plain read of this window's own slot.
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const Shared;
    if ptr.is_null() {
        // SAFETY: the default handling, with the message as it came.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // SAFETY: a non-null slot holds `Rc::as_ptr` of the `TrayIcon` that owns this window, whose `Drop` clears it
    // before that `Rc` can go. Taking a count of our own keeps the state alive if the icon is dropped during this
    // call (a quit picked while the menu is open).
    let shared = unsafe {
        Rc::increment_strong_count(ptr);
        Rc::from_raw(ptr)
    };
    shared.handle(hwnd, msg, wparam, lparam)
}

/// An `HICON` this module made.
struct Icon(HICON);

impl Drop for Icon {
    fn drop(&mut self) {
        // SAFETY: made by `CreateIconIndirect` and owned only here.
        let _ = unsafe { DestroyIcon(self.0) };
    }
}

/// A bitmap freed when it goes; `CreateIconIndirect` copies both of its bitmaps.
struct Bitmap(HBITMAP);

impl Drop for Bitmap {
    fn drop(&mut self) {
        // SAFETY: made in `make_icon`, never selected into a DC.
        let _ = unsafe { DeleteObject(self.0.into()) };
    }
}

fn draw(want: Drawn) -> Result<Icon, WinError> {
    let mask = if want.pattern {
        pattern_mask(want.size)
    } else {
        art::tray_mask(want.icon, want.size)
    };
    make_icon(&art::colorize(&mask, want.colour))
}

/// `art::Rgba` (straight alpha, RGBA, top-down) as an `HICON`: a 32-bit top-down DIB in BGRA, and an AND mask with a
/// bit set where alpha is 0, as `art::ico` writes it.
fn make_icon(img: &art::Rgba) -> Result<Icon, WinError> {
    let (w, h) = (img.width as i32, img.height as i32);
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
    // SAFETY: `info` describes a 32-bit top-down DIB; `bits` receives the pointer to its pixels.
    let colour = unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) }
        .map_err(|e| win_error("CreateDIBSection", &e))?;
    let colour = Bitmap(colour);
    // SAFETY: the DIB section holds width * height 32-bit pixels, which is `img.pixels.len()` bytes.
    let dst = unsafe { std::slice::from_raw_parts_mut(bits.cast::<u8>(), img.pixels.len()) };
    for (d, s) in dst
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(img.pixels.as_chunks::<4>().0)
    {
        *d = [s[2], s[1], s[0], s[3]];
    }
    let mask_bits = and_mask(img);
    // SAFETY: `mask_bits` holds `h` rows of 1-bit pixels, each row padded to 16 bits, as `CreateBitmap` reads them.
    let mask = unsafe { CreateBitmap(w, h, 1, 1, Some(mask_bits.as_ptr().cast())) };
    if mask.is_invalid() {
        return Err(last_error("CreateBitmap"));
    }
    let mask = Bitmap(mask);
    let icon_info = ICONINFO {
        fIcon: true.into(),
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask.0,
        hbmColor: colour.0,
    };
    // SAFETY: both bitmaps are live; the call copies them, and they are freed when this returns.
    let icon = unsafe { CreateIconIndirect(&icon_info) }
        .map_err(|e| win_error("CreateIconIndirect", &e))?;
    Ok(Icon(icon))
}

fn and_mask(img: &art::Rgba) -> Vec<u8> {
    let stride = img.width.div_ceil(16) as usize * 2;
    let mut out = vec![0u8; stride * img.height as usize];
    for y in 0..img.height as usize {
        for x in 0..img.width as usize {
            if img.pixels[(y * img.width as usize + x) * 4 + 3] == 0 {
                out[y * stride + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    out
}

/// A one-pixel border with a one-pixel checker inside, one pixel in from it: any scaling blurs it into grey.
fn pattern_mask(size: u32) -> Mask {
    let s = size as usize;
    let mut alpha = vec![0u8; s * s];
    for y in 0..s {
        for x in 0..s {
            let edge = x == 0 || y == 0 || x == s - 1 || y == s - 1;
            let inner =
                (2..s.saturating_sub(2)).contains(&x) && (2..s.saturating_sub(2)).contains(&y);
            if edge || (inner && (x + y) % 2 == 0) {
                alpha[y * s + x] = 0xff;
            }
        }
    }
    Mask { size, alpha }
}

fn put_tip(data: &mut NOTIFYICONDATAW, tip: &str) {
    let tip = cut_tooltip(tip);
    let mut n = 0;
    for (slot, unit) in data.szTip.iter_mut().zip(tip.encode_utf16()) {
        *slot = unit;
        n += 1;
    }
    data.szTip[n] = 0;
}

/// Reads a callback message of a version 4 icon: the event in the low word of `lparam`, the icon id in its high word,
/// the anchor in `wparam` as two signed 16-bit values (screens left of or above the primary are negative).
fn decode(msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<Input> {
    if msg != WM_TRAY {
        return None;
    }
    let event = lparam.0 as u32 & 0xffff;
    let id = (lparam.0 as u32 >> 16) & 0xffff;
    if id != ICON_UID {
        return None;
    }
    let anchor = Anchor {
        x: i32::from(wparam.0 as u16 as i16),
        y: i32::from((wparam.0 >> 16) as u16 as i16),
    };
    let open = |by| Some(Input::Open { by, anchor });
    match event {
        NIN_SELECT => open(OpenBy::Click),
        NIN_KEYSELECT => open(OpenBy::Key),
        WM_CONTEXTMENU => open(OpenBy::ContextMenu),
        NIN_BALLOONUSERCLICK => Some(Input::BalloonClick),
        // Balloon show, hide and timeout are never counted: the timeout came twice in one millisecond (research,
        // "Balloon").
        _ => None,
    }
}

/// The last guard of the tooltip: `present` and `text::dry_run` already fit it, so this cuts only what slips past.
fn cut_tooltip(tip: &str) -> &str {
    let mut used = 0;
    for (i, c) in tip.char_indices() {
        used += c.len_utf16();
        if used > TIP_MAX_UTF16 {
            return &tip[..i];
        }
    }
    tip
}

/// The wait after the `attempt`-th failed add in a row: 1, 2, 4, 8, 16 s, then every 30 s. Fast at sign-in, when
/// Explorer is seconds away, and cheap when it never comes.
fn retry_delay(attempt: u32) -> Duration {
    if attempt > 5 {
        Duration::from_secs(30)
    } else {
        Duration::from_secs(1 << attempt.saturating_sub(1))
    }
}

/// The small icon size for `dpi`, which is the size of the taskbar's tray slot.
fn glyph_size(dpi: u32) -> u32 {
    // SAFETY: plain call.
    let px = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, dpi) };
    let px = match u32::try_from(px) {
        Ok(px) if px > 0 => px,
        _ => small_icon_px(dpi),
    };
    px.clamp(16, 256)
}

/// 16 px at 96 dpi, scaled and rounded as `MulDiv` does.
fn small_icon_px(dpi: u32) -> u32 {
    (16 * dpi + 48) / 96
}

/// The taskbar window's DPI (its monitor's, since Explorer is per-monitor aware), else the system DPI. Not the DPI of
/// this hidden window, which never got `WM_DPICHANGED` (research, "`LoadIconMetric` follows the system DPI").
fn taskbar_dpi() -> u32 {
    // SAFETY: plain calls; `GetDpiForWindow` checks the handle and gives 0 for a bad one.
    let dpi = unsafe { FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) }
        .map_or(0, |h| unsafe { GetDpiForWindow(h) });
    if dpi != 0 {
        dpi
    } else {
        // SAFETY: plain call.
        unsafe { GetDpiForSystem() }
    }
}

/// White on a dark taskbar, near black on a light one, and in high contrast the window text colour: Microsoft's
/// "Contrast themes" page has bars use `SystemColorWindowColor` as background, paired with
/// `SystemColorWindowTextColor`. Which colour Windows' own taskbar uses in a contrast theme: not verified.
fn glyph_colour(theme: &Theme) -> [u8; 3] {
    match theme.contrast {
        Some(c) => c.window_text.into(),
        None if theme.light => art::GLYPH_ON_LIGHT,
        None => art::GLYPH_ON_DARK,
    }
}

/// `WM_SETTINGCHANGE` for a Windows mode switch ("ImmersiveColorSet") or high contrast (`SPI_SETHIGHCONTRAST`).
fn is_theme_setting(wparam: WPARAM, lparam: LPARAM) -> bool {
    if wparam.0 as u32 == SPI_SETHIGHCONTRAST.0 {
        return true;
    }
    if lparam.0 == 0 {
        return false;
    }
    let name = PCWSTR(lparam.0 as *const u16);
    // SAFETY: a non-zero `lParam` of `WM_SETTINGCHANGE` points at the sender's NUL-terminated section name, valid for
    // this call.
    unsafe { name.as_wide() }
        .iter()
        .copied()
        .eq("ImmersiveColorSet".encode_utf16())
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
    use crate::tray::theme::{Contrast, Rgb};
    use wakeshot_core::view::{MenuCommand, MenuItem, MenuModel};
    use windows::Win32::Graphics::Gdi::{
        BITMAP, CreateCompatibleDC, DeleteDC, GetDIBits, GetObjectW,
    };
    use windows::Win32::UI::Shell::{NIN_BALLOONHIDE, NIN_BALLOONSHOW, NIN_BALLOONTIMEOUT};
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, SendMessageW};

    fn wp(x: i16, y: i16) -> WPARAM {
        WPARAM((x as u16 as usize) | ((y as u16 as usize) << 16))
    }

    fn lp(event: u32, id: u32) -> LPARAM {
        LPARAM(((id << 16) | event) as isize)
    }

    #[test]
    fn decode_version_4_messages() {
        let at = Anchor { x: -1500, y: 2100 };
        let w = wp(-1500, 2100);
        let open = |by| Some(Input::Open { by, anchor: at });
        assert_eq!(decode(WM_TRAY, w, lp(NIN_SELECT, 0)), open(OpenBy::Click));
        assert_eq!(decode(WM_TRAY, w, lp(NIN_KEYSELECT, 0)), open(OpenBy::Key));
        assert_eq!(
            decode(WM_TRAY, w, lp(WM_CONTEXTMENU, 0)),
            open(OpenBy::ContextMenu)
        );
        assert_eq!(
            decode(WM_TRAY, w, lp(NIN_BALLOONUSERCLICK, 0)),
            Some(Input::BalloonClick)
        );
        for ignored in [NIN_BALLOONSHOW, NIN_BALLOONHIDE, NIN_BALLOONTIMEOUT, 0x200] {
            assert_eq!(decode(WM_TRAY, w, lp(ignored, 0)), None, "{ignored:#x}");
        }
        assert_eq!(decode(WM_TRAY, w, lp(NIN_SELECT, 7)), None, "other icon id");
        assert_eq!(decode(WM_APP, w, lp(NIN_SELECT, 0)), None, "other message");
    }

    #[test]
    fn tooltip_cut_keeps_whole_characters() {
        // 126 units, then a surrogate pair that would end at 128.
        let tip = format!("{}\u{1F600}", "a".repeat(126));
        assert_eq!(cut_tooltip(&tip), "a".repeat(126));
        let tip = format!("{}\u{1F600}", "a".repeat(125));
        assert_eq!(cut_tooltip(&tip), tip, "ends exactly at 127");
        let armenian = "Ա".repeat(200);
        let cut = cut_tooltip(&armenian);
        assert_eq!(cut.encode_utf16().count(), 127);
    }

    #[test]
    fn short_tooltip_is_unchanged() {
        let tip = "Wakeshot\nIdle: the PC sleeps as usual";
        assert_eq!(cut_tooltip(tip), tip);
        assert_eq!(cut_tooltip(""), "");
    }

    #[test]
    fn retry_backs_off_then_holds_at_30_s() {
        let secs: Vec<u64> = (1..=9).map(|n| retry_delay(n).as_secs()).collect();
        assert_eq!(secs, [1, 2, 4, 8, 16, 30, 30, 30, 30]);
        assert_eq!(retry_delay(0).as_secs(), 1);
        assert_eq!(retry_delay(u32::MAX).as_secs(), 30);
    }

    #[test]
    fn glyph_size_per_dpi() {
        for (dpi, px) in [(96, 16), (120, 20), (144, 24), (192, 32)] {
            assert_eq!(glyph_size(dpi), px, "{dpi} dpi");
        }
        // The fallback agrees with Windows at every 25% step from 100% to 400%.
        for step in 4..=16 {
            let dpi = step * 24;
            assert_eq!(small_icon_px(dpi), glyph_size(dpi), "{dpi} dpi");
        }
    }

    fn contrast(text: Rgb) -> Contrast {
        let black = Rgb::new(0, 0, 0);
        Contrast {
            window: black,
            window_text: text,
            highlight: black,
            highlight_text: black,
            gray_text: black,
            button_face: black,
            button_text: black,
            hot_light: black,
        }
    }

    #[test]
    fn glyph_colour_per_theme() {
        let dark = Theme {
            light: false,
            contrast: None,
        };
        let light = Theme {
            light: true,
            contrast: None,
        };
        assert_eq!(glyph_colour(&dark), [0xff, 0xff, 0xff]);
        assert_eq!(glyph_colour(&light), [0x1a, 0x1a, 0x1a]);
        let yellow = Rgb::new(0xff, 0xff, 0x00);
        for light in [false, true] {
            let theme = Theme {
                light,
                contrast: Some(contrast(yellow)),
            };
            assert_eq!(glyph_colour(&theme), [0xff, 0xff, 0x00]);
        }
    }

    #[test]
    fn hicon_from_art_reads_back() {
        let size = 24;
        let img = art::colorize(
            &art::tray_mask(IconState::Holding, size),
            [0x12, 0x34, 0x56],
        );
        let icon = make_icon(&img).expect("icon");
        let mut info = ICONINFO::default();
        // SAFETY: a live icon and a valid out pointer; the bitmaps it returns are freed below.
        unsafe { GetIconInfo(icon.0, &mut info) }.expect("GetIconInfo");
        let colour = Bitmap(info.hbmColor);
        let _mask = Bitmap(info.hbmMask);
        let mut bm = BITMAP::default();
        // SAFETY: a live bitmap and a BITMAP-sized out buffer.
        let n = unsafe {
            GetObjectW(
                colour.0.into(),
                size_of::<BITMAP>() as i32,
                Some((&raw mut bm).cast()),
            )
        };
        assert!(n > 0);
        assert_eq!((bm.bmWidth, bm.bmHeight), (size as i32, size as i32));
        assert_eq!(bm.bmBitsPixel, 32);

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size as i32,
                biHeight: -(size as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut px = vec![0u8; (size * size * 4) as usize];
        // SAFETY: a memory DC made and freed here; `px` holds `size` rows of 32-bit pixels as `bmi` says.
        let rows = unsafe {
            let dc = CreateCompatibleDC(None);
            let rows = GetDIBits(
                dc,
                colour.0,
                0,
                size,
                Some(px.as_mut_ptr().cast()),
                &mut bmi,
                DIB_RGB_COLORS,
            );
            let _ = DeleteDC(dc);
            rows
        };
        assert_eq!(rows, size as i32);
        let opaque = img
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .position(|p| p[3] == 0xff)
            .expect("an opaque pixel");
        let got = &px[opaque * 4..opaque * 4 + 4];
        assert_eq!(got, [0x56, 0x34, 0x12, 0xff], "BGRA, alpha kept");
        let clear = img
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .position(|p| p[3] == 0)
            .unwrap();
        assert_eq!(px[clear * 4 + 3], 0);
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Call {
        op: NOTIFY_ICON_MESSAGE,
        icon: bool,
        tip: String,
        guid: u128,
    }

    #[derive(Clone, Default)]
    struct FakeShell {
        calls: Rc<RefCell<Vec<Call>>>,
        fail_adds: Rc<Cell<u32>>,
        fail_modifies: Rc<Cell<u32>>,
    }

    impl FakeShell {
        fn ops(&self) -> Vec<NOTIFY_ICON_MESSAGE> {
            self.calls.borrow().iter().map(|c| c.op).collect()
        }

        fn clear(&self) {
            self.calls.borrow_mut().clear();
        }
    }

    impl Shell for FakeShell {
        fn notify(&self, op: NOTIFY_ICON_MESSAGE, data: &NOTIFYICONDATAW) -> bool {
            let end = data.szTip.iter().position(|&u| u == 0).unwrap_or(128);
            self.calls.borrow_mut().push(Call {
                op,
                icon: data.uFlags.contains(NIF_ICON) && !data.hIcon.is_invalid(),
                tip: String::from_utf16_lossy(&data.szTip[..end]),
                guid: data.guidItem.to_u128(),
            });
            let fail = |n: &Cell<u32>| {
                let left = n.get();
                n.set(left.saturating_sub(1));
                left > 0
            };
            match op {
                NIM_ADD => !fail(&self.fail_adds),
                NIM_MODIFY => !fail(&self.fail_modifies),
                _ => true,
            }
        }
    }

    #[derive(Clone, Default)]
    struct Recorder(Rc<RefCell<Vec<Anchor>>>);

    impl MenuView for Recorder {
        fn show(&mut self, _menu: &MenuModel, anchor: Anchor) {
            self.0.borrow_mut().push(anchor);
        }
    }

    fn view(icon: IconState, tooltip: &str) -> TrayView {
        TrayView {
            icon,
            tooltip: tooltip.to_owned(),
            menu: MenuModel {
                items: vec![MenuItem::Action {
                    text: "&Quit".to_owned(),
                    command: MenuCommand::Quit,
                    checked: false,
                    enabled: true,
                }],
            },
        }
    }

    fn icon(kind: IconKind, menu: Box<dyn MenuView>, shell: &FakeShell) -> TrayIcon {
        TrayIcon::with_shell(kind, menu, Box::new(shell.clone())).expect("tray icon")
    }

    fn click(hwnd: HWND, event: u32) {
        // SAFETY: this thread's own window; `SendMessageW` calls its procedure directly.
        unsafe { SendMessageW(hwnd, WM_TRAY, Some(wp(10, 20)), Some(lp(event, 0))) };
    }

    #[test]
    fn first_show_adds_then_modifies_only_on_change() {
        let shell = FakeShell::default();
        let mut tray = icon(IconKind::Product, Box::new(Recorder::default()), &shell);
        assert!(shell.ops().is_empty(), "nothing before the first view");
        tray.show(&view(IconState::Idle, "Wakeshot\nIdle"));
        assert_eq!(shell.ops(), [NIM_ADD, NIM_SETVERSION]);
        let add = shell.calls.borrow()[0].clone();
        assert!(add.icon);
        assert_eq!(add.tip, "Wakeshot\nIdle");
        assert_eq!(add.guid, IconKind::Product.guid());
        shell.clear();

        tray.show(&view(IconState::Idle, "Wakeshot\nIdle"));
        assert!(shell.ops().is_empty(), "same view, no call");

        tray.show(&view(IconState::Idle, "Wakeshot\nIdle now"));
        let calls = shell.calls.borrow().clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].op, NIM_MODIFY);
        assert!(!calls[0].icon, "tooltip only");
        shell.clear();

        tray.show(&view(IconState::Holding, "Wakeshot\nIdle now"));
        let calls = shell.calls.borrow().clone();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].icon, "new state, new icon");
        shell.clear();

        drop(tray);
        assert_eq!(shell.ops(), [NIM_DELETE]);
    }

    #[test]
    fn forced_theme_redraws_only_on_a_colour_change() {
        let shell = FakeShell::default();
        let mut tray = icon(IconKind::Demo, Box::new(Recorder::default()), &shell);
        let notes = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&notes);
        tray.on_note(Box::new(move |n| seen.borrow_mut().push(n.clone())));
        tray.force_theme(Some(Theme {
            light: false,
            contrast: None,
        }));
        tray.show(&view(IconState::Idle, "t"));
        shell.clear();
        tray.force_theme(Some(Theme {
            light: false,
            contrast: None,
        }));
        assert!(shell.ops().is_empty());
        tray.force_theme(Some(Theme {
            light: true,
            contrast: None,
        }));
        assert_eq!(shell.ops(), [NIM_MODIFY]);
        assert!(shell.calls.borrow()[0].icon);
        let last = notes.borrow().last().cloned();
        assert!(
            matches!(last, Some(Note::Redrawn { colour, .. }) if colour == art::GLYPH_ON_LIGHT),
            "{last:?}"
        );
    }

    #[test]
    fn failed_add_retries_then_taskbar_created_re_adds() {
        let shell = FakeShell::default();
        shell.fail_adds.set(2);
        let mut tray = icon(IconKind::Product, Box::new(Recorder::default()), &shell);
        let notes = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&notes);
        tray.on_note(Box::new(move |n| seen.borrow_mut().push(n.clone())));
        tray.show(&view(IconState::Idle, "t"));
        // Both tries of the attempt fail: add, delete, add.
        assert_eq!(shell.ops(), [NIM_ADD, NIM_DELETE, NIM_ADD]);
        assert_eq!(
            notes.borrow().last(),
            Some(&Note::AddFailed {
                attempt: 1,
                retry_in: Duration::from_secs(1)
            })
        );
        shell.clear();

        // The retry timer fires.
        // SAFETY: this thread's own window; `SendMessageW` calls its procedure directly.
        unsafe { SendMessageW(tray.hwnd, WM_TIMER, Some(WPARAM(TIMER_RETRY)), None) };
        assert_eq!(shell.ops(), [NIM_ADD, NIM_SETVERSION]);
        assert_eq!(notes.borrow().last(), Some(&Note::Added { attempt: 2 }));
        shell.clear();

        // Explorer restarted: the icon is added again, with version 4 and the last view.
        let tc = tray.shared.taskbar_created;
        assert_ne!(tc, 0);
        // SAFETY: as above.
        unsafe { SendMessageW(tray.hwnd, tc, None, None) };
        assert_eq!(shell.ops(), [NIM_ADD, NIM_SETVERSION]);
        assert_eq!(shell.calls.borrow()[0].tip, "t");
        shell.clear();

        // A modify that fails means the icon is gone: it is added again at once.
        shell.fail_modifies.set(1);
        tray.show(&view(IconState::Holding, "t"));
        assert_eq!(shell.ops(), [NIM_MODIFY, NIM_ADD, NIM_SETVERSION]);
    }

    #[test]
    fn dry_run_marks_the_tooltip() {
        let shell = FakeShell::default();
        let mut tray = icon(IconKind::DryRun, Box::new(Recorder::default()), &shell);
        let long = format!("Wakeshot\n{}", "x".repeat(200));
        tray.show(&view(IconState::Idle, &long));
        let add = shell.calls.borrow()[0].clone();
        assert!(add.tip.starts_with("Wakeshot (dry run)\n"), "{}", add.tip);
        assert!(add.tip.encode_utf16().count() <= TIP_MAX_UTF16);
        assert_eq!(add.guid, IconKind::DryRun.guid());
        assert_eq!(tray.identity().guid, IconKind::DryRun.guid());
        assert_eq!(tray.identity().hwnd, tray.hwnd.0 as isize);
    }

    #[test]
    fn clicks_open_the_menu_and_balloon_click_calls_the_hook() {
        let shell = FakeShell::default();
        let menu = Recorder::default();
        let mut tray = icon(IconKind::Demo, Box::new(menu.clone()), &shell);
        click(tray.hwnd, NIN_SELECT);
        assert!(menu.0.borrow().is_empty(), "no view yet, no menu");
        tray.show(&view(IconState::Idle, "t"));
        let clicks = Rc::new(Cell::new(0));
        let count = Rc::clone(&clicks);
        tray.on_balloon_click(Box::new(move || count.set(count.get() + 1)));
        for event in [NIN_SELECT, NIN_KEYSELECT, WM_CONTEXTMENU] {
            click(tray.hwnd, event);
        }
        assert_eq!(*menu.0.borrow(), [Anchor { x: 10, y: 20 }; 3]);
        for event in [
            NIN_BALLOONSHOW,
            NIN_BALLOONTIMEOUT,
            NIN_BALLOONTIMEOUT,
            NIN_BALLOONHIDE,
        ] {
            click(tray.hwnd, event);
        }
        assert_eq!(clicks.get(), 0, "balloon events are not clicks");
        click(tray.hwnd, NIN_BALLOONUSERCLICK);
        assert_eq!(clicks.get(), 1);
    }

    /// Opens like the real menu: during `show`, the loop calls `TrayIcon::show` with a new view, and a second click
    /// arrives.
    struct CallsBack {
        slot: Rc<RefCell<Option<TrayIcon>>>,
        shell: FakeShell,
        opened: Rc<Cell<u32>>,
    }

    impl MenuView for CallsBack {
        fn show(&mut self, menu: &MenuModel, _anchor: Anchor) {
            self.opened.set(self.opened.get() + 1);
            assert_eq!(menu.items.len(), 1, "the menu of the view before");
            let hwnd = self.slot.borrow().as_ref().expect("icon").hwnd;
            let mut next = view(IconState::Holding, "held");
            next.menu = MenuModel::default();
            self.slot.borrow_mut().as_mut().expect("icon").show(&next);
            click(hwnd, WM_CONTEXTMENU);
            assert!(
                self.shell.ops().is_empty(),
                "no shell call while the menu is open"
            );
        }
    }

    #[test]
    fn show_during_menu_is_queued() {
        let shell = FakeShell::default();
        let slot = Rc::new(RefCell::new(None));
        let opened = Rc::new(Cell::new(0));
        let menu = CallsBack {
            slot: Rc::clone(&slot),
            shell: shell.clone(),
            opened: Rc::clone(&opened),
        };
        let mut tray = icon(IconKind::Demo, Box::new(menu), &shell);
        tray.show(&view(IconState::Idle, "idle"));
        let hwnd = tray.hwnd;
        *slot.borrow_mut() = Some(tray);
        shell.clear();

        click(hwnd, NIN_SELECT);
        assert_eq!(opened.get(), 1, "the click during the menu did nothing");
        let calls = shell.calls.borrow().clone();
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].op, NIM_MODIFY);
        assert!(calls[0].icon);
        assert_eq!(calls[0].tip, "held");
        // The queued view's menu is the one for the next click.
        let state = slot.borrow();
        let st = state.as_ref().unwrap().shared.state.borrow();
        assert!(st.view.as_ref().unwrap().menu.items.is_empty());
        drop(st);
        drop(state);
        slot.borrow_mut().take();
    }

    #[test]
    fn pattern_has_a_border_and_a_checker() {
        let m = pattern_mask(20);
        let at = |x: usize, y: usize| m.alpha[y * 20 + x];
        assert!((0..20).all(|i| at(i, 0) == 0xff && at(0, i) == 0xff && at(19, i) == 0xff));
        assert_eq!(at(1, 1), 0, "gap inside the border");
        assert_eq!((at(2, 2), at(3, 2)), (0xff, 0));
    }

    #[test]
    fn and_mask_marks_clear_pixels() {
        let img = art::Rgba {
            width: 17,
            height: 1,
            pixels: (0..17)
                .flat_map(|x| [0, 0, 0, if x == 0 { 0 } else { 0xff }])
                .collect(),
        };
        assert_eq!(and_mask(&img), [0x80, 0, 0, 0], "17 px pad to 32 bits");
    }

    #[test]
    fn theme_settings_are_recognised() {
        let name: Vec<u16> = "ImmersiveColorSet\0".encode_utf16().collect();
        let other: Vec<u16> = "Environment\0".encode_utf16().collect();
        assert!(is_theme_setting(WPARAM(0), LPARAM(name.as_ptr() as isize)));
        assert!(!is_theme_setting(
            WPARAM(0),
            LPARAM(other.as_ptr() as isize)
        ));
        assert!(!is_theme_setting(WPARAM(0), LPARAM(0)));
        assert!(is_theme_setting(
            WPARAM(SPI_SETHIGHCONTRAST.0 as usize),
            LPARAM(0)
        ));
    }
}
