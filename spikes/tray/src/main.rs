// Phase 0 spike: tray icon, themed menu, notifications, sound, footprint. Throwaway code; see README.md.
#![windows_subsystem = "windows"]

mod capture;
// The taskbar colours are used by build.rs only.
#[allow(dead_code)]
mod glyph;
mod log;
mod look;
mod sound;
mod theme;
mod toast;
mod token;

use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
};

use windows::{
    Win32::{
        Foundation::{COLORREF, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BITMAP, CreateSolidBrush, DeleteObject, FillRect, GetObjectW, HDC},
        System::{
            LibraryLoader::GetModuleHandleW,
            ProcessStatus::{
                GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
            },
            Threading::GetCurrentProcess,
        },
        UI::{
            Controls::{DRAWITEMSTRUCT, LIM_SMALL, LoadIconMetric, MEASUREITEMSTRUCT, ODT_MENU},
            HiDpi::{
                AreDpiAwarenessContextsEqual, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
                GetDpiForSystem, GetSystemMetricsForDpi, GetThreadDpiAwarenessContext,
            },
            Input::KeyboardAndMouse::VK_DOWN,
            Shell::{
                NIF_GUID, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_INFO,
                NIIF_NOSOUND, NIIF_RESPECT_QUIET_TIME, NIM_ADD, NIM_DELETE, NIM_MODIFY,
                NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, NOTIFYICONIDENTIFIER,
                Shell_NotifyIconGetRect, Shell_NotifyIconW, ShellExecuteW,
            },
            WindowsAndMessaging::*,
        },
    },
    core::{GUID, HSTRING, PCWSTR, w},
};

use look::{Entry, Kind};
use theme::{AppMode, UxTheme};

const WM_TRAY: u32 = WM_APP + 1;
const TRAY_UID: u32 = 1;
// A fixed random GUID for the NIF_GUID path test only.
const SPIKE_GUID: GUID = GUID::from_u128(0x5e1f3a2c_7b9d_4c1e_9a6f_2d8b4e0c7a13);

// Must match build.rs.
const ICON_HOLD_ON_DARK: u16 = 101;
const ICON_IDLE_ON_DARK: u16 = 102;
const ICON_HOLD_ON_LIGHT: u16 = 103;
const ICON_IDLE_ON_LIGHT: u16 = 104;

const TIMER_AUTO: usize = 1;
const TIMER_MENU_CAPTURE: usize = 2;
const TIMER_THEME_SETTLE: usize = 3;
const TIMER_MENU_WATCH: usize = 4;
const TIMER_MEM: usize = 5;
const TIMER_ADD_RETRY: usize = 6;
const TIMER_END: usize = 7;
const TIMER_RECT: usize = 8;

const ID_STATUS: u32 = 1000;
const ID_AUTOMATIC: u32 = 1001;
const ID_KEEP_BASE: u32 = 1010;
const ID_SCREEN_ON: u32 = 1020;
const ID_BALLOON: u32 = 1030;
const ID_TOAST_REG: u32 = 1031;
const ID_TOAST_UNREG: u32 = 1032;
const ID_SOUND_ALIAS: u32 = 1033;
const ID_SOUND_BUNDLED: u32 = 1034;
const ID_RESTORE: u32 = 1040;
const ID_OPEN_LOG: u32 = 1041;
const ID_QUIT: u32 = 1042;
const KEEP_LABELS: [&str; 6] = [
    "Off",
    "30 minutes",
    "1 hour",
    "2 hours",
    "4 hours",
    "Until I turn it off",
];
const MAX_LIVE_CAPTURES: u32 = 12;

// Test messages for the look card's scripts. Style: wParam bit 0 owner-drawn, bit 1 forced high contrast,
// bits 8 and up the corner bits of `look` (1 first paint, 2 idle, 4 border colour). Capture: wParam bit 0 submenu, bits 4 to 7 Down presses, lParam
// a screen point (x low word, y high word) on the monitor to open on, 0 for the primary.
const WM_LOOK_STYLE: u32 = WM_APP + 2;
const WM_LOOK_CAPTURE: u32 = WM_APP + 3;
const WM_LOOK_DROP_ICON: u32 = WM_APP + 4;
// wParam 0 hides, 1 light, 2 dark; lParam a screen point as above.
const WM_LOOK_BACKDROP: u32 = WM_APP + 5;
const WM_LOOK_SHOT: u32 = WM_APP + 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Interactive,
    Auto,
    Menus,
    Idle(u32),
    GuidTest,
}

// Cells, not one RefCell: TrackPopupMenu and Shell_NotifyIcon re-enter the window procedure,
// and a held borrow would then panic.
struct State {
    mode: Cell<Mode>,
    hwnd: Cell<HWND>,
    msg_hwnd: Cell<HWND>,
    module: Cell<HMODULE>,
    ux: Cell<Option<UxTheme>>,
    taskbar_created: Cell<u32>,
    icon: Cell<HICON>,
    holding: Cell<bool>,
    keep: Cell<u32>,
    screen_on: Cell<bool>,
    add_attempts: Cell<u32>,
    auto_step: Cell<u32>,
    capture_tag: RefCell<String>,
    capture_submenu: Cell<bool>,
    capture_stage: Cell<u8>,
    watch_seen: Cell<usize>,
    watch_saved: Cell<usize>,
    live_captures: Cell<u32>,
    switches: Cell<u32>,
    log_name: Cell<&'static str>,
    owner_drawn: Cell<bool>,
    force_hc: Cell<bool>,
    corners: Cell<u32>,
    look: Cell<bool>,
    capture_downs: Cell<u32>,
    capture_at: Cell<Option<(i32, i32)>>,
    backdrop: Cell<HWND>,
    backdrop_colour: Cell<u32>,
}

thread_local! {
    static S: State = State {
        mode: Cell::new(Mode::Interactive),
        hwnd: Cell::new(HWND::default()),
        msg_hwnd: Cell::new(HWND::default()),
        module: Cell::new(HMODULE::default()),
        ux: Cell::new(None),
        taskbar_created: Cell::new(0),
        icon: Cell::new(HICON::default()),
        holding: Cell::new(true),
        keep: Cell::new(0),
        screen_on: Cell::new(false),
        add_attempts: Cell::new(0),
        auto_step: Cell::new(0),
        capture_tag: RefCell::new(String::new()),
        capture_submenu: Cell::new(false),
        capture_stage: Cell::new(0),
        watch_seen: Cell::new(0),
        watch_saved: Cell::new(0),
        live_captures: Cell::new(0),
        switches: Cell::new(0),
        log_name: Cell::new("log-interactive.txt"),
        owner_drawn: Cell::new(true),
        force_hc: Cell::new(false),
        corners: Cell::new(look::CORNERS_AT_PAINT | look::CORNERS_AT_IDLE),
        look: Cell::new(false),
        capture_downs: Cell::new(0),
        capture_at: Cell::new(None),
        backdrop: Cell::new(HWND::default()),
        backdrop_colour: Cell::new(0),
    };
}

fn st<R>(f: impl FnOnce(&State) -> R) -> R {
    S.with(f)
}
fn hwnd() -> HWND {
    st(|s| s.hwnd.get())
}
fn ux() -> UxTheme {
    st(|s| s.ux.get()).expect("uxtheme loaded at start")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match args.first().map(String::as_str) {
        Some("--auto") => Mode::Auto,
        Some("--menus") => Mode::Menus,
        Some("--idle") => Mode::Idle(args.get(1).and_then(|v| v.parse().ok()).unwrap_or(30)),
        Some("--guid-test") => Mode::GuidTest,
        Some("--render") => {
            log::init("log-render.txt");
            look::render_all(&render_menus(), &log::path("look"));
            return;
        }
        Some("--cleanup") => {
            log::init("log-cleanup.txt");
            log!("cleanup: {}", toast::unregister());
            return;
        }
        // Lets a launcher check which token a start from it gives, with no icon or keys.
        Some("--token") => {
            log::init("log-token.txt");
            let tok = token::info();
            log!(
                "token: pid {}, elevated {}, integrity {} (0x{:x})",
                std::process::id(),
                tok.elevated,
                tok.integrity_name(),
                tok.integrity_rid
            );
            return;
        }
        _ => Mode::Interactive,
    };
    let look_run = args.iter().any(|a| a == "--look");
    st(|s| {
        s.owner_drawn.set(!args.iter().any(|a| a == "--classic"));
        s.look.set(look_run);
    });
    let log_name = match mode {
        Mode::Interactive if look_run => "log-look.txt",
        Mode::Interactive => "log-interactive.txt",
        Mode::Auto => "log-auto.txt",
        Mode::Menus => "log-menus.txt",
        Mode::Idle(_) => "log-idle.txt",
        Mode::GuidTest => "log-guid.txt",
    };
    log::init(log_name);
    st(|s| s.log_name.set(log_name));
    let tok = token::info();
    log!(
        "token: pid {}, elevated {}, integrity {} (0x{:x})",
        std::process::id(),
        tok.elevated,
        tok.integrity_name(),
        tok.integrity_rid
    );
    if mode == Mode::Interactive && tok.elevated && !args.iter().any(|a| a == "--allow-elevated") {
        log!("refusing: elevated token");
        unsafe {
            MessageBoxW(
                None,
                w!(
                    "This tray spike runs elevated, so the Explorer restart test would be wrong.\n\nStart it from the morning shell (spikes\\morning-shell.cmd, opened from File Explorer)."
                ),
                w!("Wakeshot tray spike"),
                MB_OK | MB_ICONWARNING,
            );
        }
        return;
    }
    run(mode);
}

fn run(mode: Mode) {
    let module = unsafe { GetModuleHandleW(None) }.expect("module handle");
    st(|s| {
        s.mode.set(mode);
        s.module.set(module);
    });

    // Must run before the first menu is created, or the menu keeps the light theme.
    let ux = UxTheme::load();
    let prev = ux.set_mode(AppMode::AllowDark);
    ux.flush();
    st(|s| s.ux.set(Some(ux)));

    let class = w!("WakeshotTraySpike");
    let wc = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(wndproc),
        hInstance: module.into(),
        lpszClassName: class,
        ..Default::default()
    };
    unsafe { RegisterClassExW(&wc) };
    // A hidden top-level window: message-only windows do not get broadcasts such as TaskbarCreated.
    // The message-only one exists to prove that.
    let main = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class,
            w!("Wakeshot tray spike"),
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
    .expect("main window");
    let msg_only = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!("Wakeshot tray spike message-only"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(module.into()),
            None,
        )
    }
    .expect("message-only window");
    let tc = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    st(|s| {
        s.hwnd.set(main);
        s.msg_hwnd.set(msg_only);
        s.taskbar_created.set(tc);
    });
    log_env(prev);

    if mode == Mode::GuidTest {
        guid_icon(NIM_ADD);
        timer(TIMER_END, 3000);
    } else if !add_icon("start") {
        timer(TIMER_ADD_RETRY, 2000);
    }
    match mode {
        Mode::Auto => timer(TIMER_AUTO, 1500),
        Mode::Menus => {
            st(|s| s.auto_step.set(3));
            timer(TIMER_AUTO, 1500);
        }
        Mode::Idle(secs) => timer(TIMER_MEM, secs * 1000),
        Mode::Interactive => {
            timer(TIMER_MEM, 10_000);
            timer(TIMER_RECT, 1500);
        }
        Mode::GuidTest => {}
    }

    let mut msg = MSG::default();
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    log!("message loop ended");
}

fn timer(id: usize, ms: u32) {
    unsafe { SetTimer(Some(hwnd()), id, ms, None) };
}
fn kill_timer(id: usize) {
    unsafe {
        let _ = KillTimer(Some(hwnd()), id);
    }
}

fn log_env(prev_mode: Option<i32>) {
    log!(
        "exe {}",
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    );
    log!("os {}", theme::os_build());
    let pmv2 = unsafe {
        AreDpiAwarenessContextsEqual(
            GetThreadDpiAwarenessContext(),
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        )
    }
    .as_bool();
    log!(
        "dpi: per-monitor v2 from manifest={pmv2}, system dpi={}",
        unsafe { GetDpiForSystem() }
    );
    for (r, dpi) in theme::monitors() {
        let sm = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, dpi) };
        log!("monitor {r:?} effective dpi {dpi}, SM_CXSMICON at that dpi = {sm}");
    }
    log!(
        "SM_CXSMICON={} SM_MENUDROPALIGNMENT={}",
        unsafe { GetSystemMetrics(SM_CXSMICON) },
        unsafe { GetSystemMetrics(SM_MENUDROPALIGNMENT) }
    );
    let ux = ux();
    log!("uxtheme: {}", ux.found());
    log!("SetPreferredAppMode(AllowDark) returned previous mode {prev_mode:?}");
    log_theme("start");
}

fn log_theme(when: &str) {
    log!(
        "theme ({when}): AppsUseLightTheme={:?} SystemUsesLightTheme={:?} ShouldAppsUseDarkMode={:?} high contrast={:?}",
        theme::apps_light(),
        theme::taskbar_light(),
        ux().apps_dark(),
        theme::high_contrast()
    );
}

fn mem(label: &str) {
    let mut c = PROCESS_MEMORY_COUNTERS_EX {
        cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ..Default::default()
    };
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut c as *mut PROCESS_MEMORY_COUNTERS_EX as *mut PROCESS_MEMORY_COUNTERS,
            c.cb,
        )
    };
    match ok {
        Ok(()) => log!(
            "memory ({label}): working set {} KB, peak working set {} KB, private {} KB",
            c.WorkingSetSize / 1024,
            c.PeakWorkingSetSize / 1024,
            c.PrivateUsage / 1024
        ),
        Err(e) => log!("memory ({label}): {e}"),
    }
}

// ---------- tray icon ----------

fn wide<const N: usize>(dst: &mut [u16; N], s: &str) {
    let v: Vec<u16> = s.encode_utf16().take(N - 1).collect();
    dst[..v.len()].copy_from_slice(&v);
    dst[v.len()] = 0;
}

fn nid() -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd(),
        uID: TRAY_UID,
        ..Default::default()
    }
}

fn tooltip() -> &'static str {
    if st(|s| s.holding.get()) {
        "Wakeshot spike\nHolding awake: Claude Code, 2 sessions"
    } else {
        "Wakeshot spike\nIdle: the PC sleeps as usual"
    }
}

/// Loads the glyph that suits the taskbar colour, sized for the current DPI by LoadIconMetric.
fn load_icon() -> HICON {
    let holding = st(|s| s.holding.get());
    let light = theme::taskbar_light().unwrap_or(false);
    let id = match (holding, light) {
        (true, false) => ICON_HOLD_ON_DARK,
        (false, false) => ICON_IDLE_ON_DARK,
        (true, true) => ICON_HOLD_ON_LIGHT,
        (false, true) => ICON_IDLE_ON_LIGHT,
    };
    let module = st(|s| s.module.get());
    match unsafe {
        LoadIconMetric(
            Some(module.into()),
            PCWSTR(id as usize as *const u16),
            LIM_SMALL,
        )
    } {
        Ok(h) => {
            log!("LoadIconMetric(id {id}, LIM_SMALL): {} px", icon_size(h));
            h
        }
        Err(e) => {
            log!("LoadIconMetric(id {id}) failed: {e}");
            HICON::default()
        }
    }
}

fn icon_size(h: HICON) -> i32 {
    let mut ii = ICONINFO::default();
    if unsafe { GetIconInfo(h, &mut ii) }.is_err() {
        return 0;
    }
    let mut bm = BITMAP::default();
    unsafe {
        GetObjectW(
            ii.hbmColor.into(),
            size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut BITMAP as *mut c_void),
        );
        let _ = DeleteObject(ii.hbmColor.into());
        let _ = DeleteObject(ii.hbmMask.into());
    }
    bm.bmWidth
}

fn swap_icon(new: HICON) {
    let old = st(|s| s.icon.replace(new));
    if !old.is_invalid() {
        // The shell keeps its own copy, so the old handle can go.
        unsafe {
            let _ = DestroyIcon(old);
        }
    }
}

fn add_icon(reason: &str) -> bool {
    let icon = load_icon();
    let mut d = nid();
    d.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    d.uCallbackMessage = WM_TRAY;
    d.hIcon = icon;
    wide(&mut d.szTip, tooltip());
    let added = unsafe { Shell_NotifyIconW(NIM_ADD, &d) }.as_bool();
    d.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    let version = added && unsafe { Shell_NotifyIconW(NIM_SETVERSION, &d) }.as_bool();
    swap_icon(icon);
    let n = st(|s| {
        s.add_attempts.set(s.add_attempts.get() + 1);
        s.add_attempts.get()
    });
    log!("NIM_ADD ({reason}, attempt {n}): {added}; NIM_SETVERSION 4: {version}");
    added
}

fn update_icon(reason: &str) {
    let icon = load_icon();
    let mut d = nid();
    d.uFlags = NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    d.hIcon = icon;
    wide(&mut d.szTip, tooltip());
    let ok = unsafe { Shell_NotifyIconW(NIM_MODIFY, &d) }.as_bool();
    swap_icon(icon);
    log!("NIM_MODIFY icon and tooltip ({reason}): {ok}");
}

fn delete_icon() {
    let d = nid();
    let ok = unsafe { Shell_NotifyIconW(NIM_DELETE, &d) }.as_bool();
    log!("NIM_DELETE: {ok}");
}

fn icon_rect() -> Result<RECT, String> {
    let id = NOTIFYICONIDENTIFIER {
        cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32,
        hWnd: hwnd(),
        uID: TRAY_UID,
        ..Default::default()
    };
    unsafe { Shell_NotifyIconGetRect(&id) }.map_err(|e| e.to_string())
}

fn guid_icon(op: windows::Win32::UI::Shell::NOTIFY_ICON_MESSAGE) {
    let mut d = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd(),
        uFlags: NIF_GUID,
        guidItem: SPIKE_GUID,
        ..Default::default()
    };
    if op == NIM_ADD {
        d.uFlags |= NIF_MESSAGE | NIF_ICON | NIF_TIP;
        d.uCallbackMessage = WM_TRAY;
        d.hIcon = load_icon();
        wide(&mut d.szTip, "Wakeshot spike (GUID test)");
    }
    let ok = unsafe { Shell_NotifyIconW(op, &d) }.as_bool();
    log!("GUID icon {op:?}: {ok}");
}

fn balloon() {
    let mut d = nid();
    d.uFlags = NIF_INFO;
    wide(&mut d.szInfoTitle, "Wakeshot spike");
    wide(
        &mut d.szInfo,
        "Balloon from Shell_NotifyIcon (NIF_INFO). No registration.",
    );
    d.dwInfoFlags = NIIF_INFO | NIIF_NOSOUND | NIIF_RESPECT_QUIET_TIME;
    let ok = unsafe { Shell_NotifyIconW(NIM_MODIFY, &d) }.as_bool();
    log!("balloon NIM_MODIFY NIF_INFO: {ok}");
}

// ---------- menu ----------

fn append(menu: HMENU, flags: MENU_ITEM_FLAGS, id: usize, text: &str) {
    let t = HSTRING::from(text);
    unsafe {
        let _ = AppendMenuW(menu, flags, id, PCWSTR(t.as_ptr()));
    }
}
fn checked(on: bool) -> MENU_ITEM_FLAGS {
    if on { MF_CHECKED } else { MF_UNCHECKED }
}

fn menu_entries() -> Vec<Entry> {
    let (holding, keep, screen_on) = st(|s| (s.holding.get(), s.keep.get(), s.screen_on.get()));
    let mut status = Entry::text(ID_STATUS, "Holding awake: Claude Code, 2 sessions");
    status.grayed = true;
    let mut automatic = Entry::text(ID_AUTOMATIC, "Automatic");
    automatic.toggle = true;
    automatic.checked = holding;
    let mut sub: Vec<Entry> = KEEP_LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let mut e = Entry::text(ID_KEEP_BASE + i as u32, label);
            e.radio = true;
            e.checked = i as u32 == keep;
            e
        })
        .collect();
    sub.push(Entry::separator());
    let mut screen = Entry::text(ID_SCREEN_ON, "Keep screen on");
    screen.toggle = true;
    screen.checked = screen_on;
    sub.push(screen);
    let mut keep_awake = Entry::text(0, "&Keep awake");
    keep_awake.kind = Kind::Submenu;
    keep_awake.sub = sub;
    vec![
        status,
        Entry::separator(),
        automatic,
        keep_awake,
        Entry::separator(),
        Entry::text(ID_BALLOON, "Test: balloon notification"),
        Entry::text(ID_TOAST_REG, "Test: toast, registered app id"),
        Entry::text(ID_TOAST_UNREG, "Test: toast, unregistered app id"),
        Entry::text(ID_SOUND_ALIAS, "Test: Windows notification sound"),
        Entry::text(ID_SOUND_BUNDLED, "Test: bundled sound"),
        Entry::separator(),
        Entry::text(ID_RESTORE, "Restore my power settings"),
        Entry::text(ID_OPEN_LOG, "Open log"),
        Entry::text(ID_QUIT, "Quit"),
    ]
}

/// The main menu and the "Keep awake" submenu, each with the index of the item drawn as hot.
fn render_menus() -> Vec<(&'static str, Vec<Entry>, usize)> {
    let mut main = menu_entries();
    let sub = std::mem::take(&mut main[3].sub);
    vec![("main", main, 2), ("sub", sub, 2)]
}

/// The classic menu as the Phase 0 spike built it, for side-by-side pictures.
fn build_classic(entries: &[Entry]) -> HMENU {
    let m = unsafe { CreatePopupMenu() }.expect("menu");
    let mut radios: Vec<(u32, bool)> = Vec::new();
    for e in entries {
        match e.kind {
            Kind::Separator => unsafe {
                let _ = AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null());
            },
            Kind::Submenu => append(m, MF_POPUP, build_classic(&e.sub).0 as usize, &e.text),
            Kind::Text => {
                let mut flags = MF_STRING | checked(e.toggle && e.checked);
                if e.grayed {
                    flags |= MF_GRAYED;
                }
                append(m, flags, e.id as usize, &e.text);
                if e.radio {
                    radios.push((e.id, e.checked));
                }
            }
        }
    }
    if let (Some(first), Some(last)) = (radios.first(), radios.last()) {
        let on = radios.iter().find(|r| r.1).map_or(first.0, |r| r.0);
        unsafe {
            let _ = CheckMenuRadioItem(m, first.0, last.0, on, MF_BYCOMMAND.0);
        }
    }
    m
}

fn show_menu(x: i32, y: i32) {
    let owner = st(|s| s.owner_drawn.get());
    let ux = ux();
    if owner {
        // Product rule: the tray menu follows the taskbar (Windows mode). The frame around the items is
        // still drawn by the theme, so it gets the same mode.
        let dark = !theme::taskbar_light().unwrap_or(false);
        let hc = st(|s| s.force_hc.get()) || theme::high_contrast().unwrap_or(false);
        ux.set_mode(if dark {
            AppMode::ForceDark
        } else {
            AppMode::ForceLight
        });
        ux.flush();
        log!("{}", look::begin(look::dpi_at(x, y), dark, hc));
    } else {
        ux.set_mode(AppMode::AllowDark);
        ux.flush();
    }
    let entries = menu_entries();
    let menu = if owner {
        let m = look::build(&entries);
        look::set_background(m);
        m
    } else {
        build_classic(&entries)
    };
    look::hook_on(st(|s| s.corners.get()));
    let h = hwnd();
    unsafe {
        // Without foreground, the menu does not close on a click outside it (KB135788).
        let fg = SetForegroundWindow(h).as_bool();
        let align = if GetSystemMetrics(SM_MENUDROPALIGNMENT) != 0 {
            TPM_RIGHTALIGN
        } else {
            TPM_LEFTALIGN
        };
        let mut flags = align | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD;
        // Owner-drawn menus need WM_MENUCHAR, so they do not ask for silence.
        if !owner {
            flags |= TPM_NONOTIFY;
        }
        let cmd = TrackPopupMenuEx(menu, flags.0, x, y, h, None).0 as u32;
        look::hook_off();
        let _ = PostMessageW(Some(h), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        look::end();
        log!("menu closed: owner-drawn {owner}, SetForegroundWindow={fg} command={cmd}");
        if cmd != 0 {
            on_command(cmd);
        }
    }
}

/// Bottom right of the work area: where a tray menu opens.
fn menu_anchor() -> (i32, i32) {
    let mut r = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut r as *mut RECT as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    (r.right - 8, r.bottom - 8)
}

fn look_tag(default: &str) -> String {
    let p = log::path("look").join("next-tag.txt");
    let tag = std::fs::read_to_string(&p)
        .map(|t| t.trim().to_string())
        .unwrap_or_default();
    let _ = std::fs::remove_file(p);
    if tag.is_empty() {
        default.to_string()
    } else {
        tag
    }
}

/// A `--look` run writes its pictures to out/look, so it never replaces the Phase 0 pictures in docs.
fn shot_path(name: &str) -> std::path::PathBuf {
    if st(|s| s.look.get()) {
        let dir = log::path("look");
        let _ = std::fs::create_dir_all(&dir);
        dir.join(name)
    } else {
        log::img_dir().join(name)
    }
}

/// Opens the menu without a click and saves screenshots.
fn capture_menu(tag: &str, submenu: bool) {
    st(|s| {
        *s.capture_tag.borrow_mut() = tag.to_string();
        s.capture_submenu.set(submenu);
        s.capture_stage.set(0);
    });
    log!(
        "programmatic menu '{tag}': ShouldAppsUseDarkMode={:?}",
        ux().apps_dark()
    );
    timer(TIMER_MENU_CAPTURE, 900);
    let (x, y) = st(|s| s.capture_at.get()).map_or_else(menu_anchor, |(x, y)| {
        let r = look::work_area_at(x, y);
        (r.right - 8, r.bottom - 8)
    });
    show_menu(x, y);
    kill_timer(TIMER_MENU_CAPTURE);
}

fn backdrop_rect() -> Option<RECT> {
    let b = st(|s| s.backdrop.get());
    let mut r = RECT::default();
    (!b.is_invalid() && unsafe { GetWindowRect(b, &mut r) }.is_ok()).then_some(r)
}

fn on_capture_timer() {
    let (tag, submenu, stage, downs) = st(|s| {
        (
            s.capture_tag.borrow().clone(),
            s.capture_submenu.get(),
            s.capture_stage.get(),
            s.capture_downs.get(),
        )
    });
    let wins = capture::menu_windows();
    if wins.is_empty() {
        log!("capture '{tag}' stage {stage}: no menu window visible");
        kill_timer(TIMER_MENU_CAPTURE);
        unsafe {
            let _ = EndMenu();
        }
        return;
    }
    if stage == 0 && downs > 0 {
        for _ in 0..downs {
            unsafe {
                let _ = PostMessageW(
                    Some(wins[0].0),
                    WM_KEYDOWN,
                    WPARAM(VK_DOWN.0 as usize),
                    LPARAM(0),
                );
            }
        }
        st(|s| s.capture_stage.set(1));
        timer(TIMER_MENU_CAPTURE, 600);
        return;
    }
    let sub_shot = stage == 2;
    let name = format!(
        "tray-menu-{tag}{}.png",
        if sub_shot { "-submenu" } else { "" }
    );
    let rects: Vec<RECT> = wins.iter().map(|w| w.1).collect();
    // A backdrop of ours behind the menu lets the picture keep the shadow without showing Haik's screen.
    let r = match backdrop_rect() {
        Some(b) => capture::capture_union(&[b], 0, &shot_path(&name)),
        None => capture::capture_union(&rects, 8, &shot_path(&name)),
    };
    log!(
        "capture '{tag}' stage {stage}: {} menu window(s) {rects:?} -> {name}: {r:?}",
        wins.len()
    );
    if !sub_shot && submenu {
        // The mnemonic opens "Keep awake" the way a keyboard user would.
        unsafe {
            let _ = PostMessageW(Some(wins[0].0), WM_CHAR, WPARAM('k' as usize), LPARAM(0));
        }
        st(|s| s.capture_stage.set(2));
        timer(TIMER_MENU_CAPTURE, 900);
    } else {
        kill_timer(TIMER_MENU_CAPTURE);
        unsafe {
            let _ = EndMenu();
        }
    }
}

extern "system" fn backdrop_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_ERASEBKGND {
        let mut r = RECT::default();
        unsafe {
            let _ = GetClientRect(h, &mut r);
            let b = CreateSolidBrush(COLORREF(st(|s| s.backdrop_colour.get())));
            FillRect(HDC(wp.0 as *mut c_void), &r, b);
            let _ = DeleteObject(b.into());
        }
        return LRESULT(1);
    }
    unsafe { DefWindowProcW(h, msg, wp, lp) }
}

/// A plain window of ours behind where menus open, so captures show shadows on a known colour.
fn backdrop(which: usize, lp: LPARAM) {
    let old = st(|s| s.backdrop.replace(HWND::default()));
    if !old.is_invalid() {
        unsafe {
            let _ = DestroyWindow(old);
        }
    }
    if which == 0 {
        log!("backdrop hidden");
        return;
    }
    let colour = if which == 1 { 0xf3f3f3 } else { 0x202020 };
    st(|s| s.backdrop_colour.set(colour));
    let (x, y) = if lp.0 == 0 {
        menu_anchor()
    } else {
        (
            (lp.0 & 0xffff) as i16 as i32,
            ((lp.0 >> 16) & 0xffff) as i16 as i32,
        )
    };
    let wa = look::work_area_at(x, y);
    let left = wa.right - (wa.right - wa.left) / 2;
    let module = st(|s| s.module.get());
    let class = w!("WakeshotLookBackdrop");
    let wc = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(backdrop_proc),
        hInstance: module.into(),
        lpszClassName: class,
        ..Default::default()
    };
    unsafe { RegisterClassExW(&wc) };
    let h = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!("Wakeshot look backdrop"),
            WS_POPUP,
            left,
            wa.top,
            wa.right - left,
            wa.bottom - wa.top,
            None,
            None,
            Some(module.into()),
            None,
        )
    };
    match h {
        Ok(h) => unsafe {
            let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                h,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            );
            st(|s| s.backdrop.set(h));
            log!(
                "backdrop {colour:06x} at {left},{} to {},{}",
                wa.top,
                wa.right,
                wa.bottom
            );
        },
        Err(e) => log!("backdrop: {e}"),
    }
}

fn on_look_message(msg: u32, wp: WPARAM, lp: LPARAM) {
    match msg {
        WM_LOOK_STYLE => {
            st(|s| {
                s.owner_drawn.set(wp.0 & 1 != 0);
                s.force_hc.set(wp.0 & 2 != 0);
                s.corners.set(((wp.0 >> 8) & 0xff) as u32);
            });
            log!(
                "look style: owner-drawn {}, forced high contrast {}, corner bits {}",
                wp.0 & 1 != 0,
                wp.0 & 2 != 0,
                (wp.0 >> 8) & 0xff
            );
        }
        WM_LOOK_CAPTURE => {
            let at = (lp.0 != 0).then_some((
                (lp.0 & 0xffff) as i16 as i32,
                ((lp.0 >> 16) & 0xffff) as i16 as i32,
            ));
            st(|s| {
                s.capture_at.set(at);
                s.capture_downs.set(((wp.0 >> 4) & 0xf) as u32);
            });
            let tag = look_tag("look");
            capture_menu(&tag, wp.0 & 1 != 0);
            st(|s| {
                s.capture_at.set(None);
                s.capture_downs.set(0);
            });
        }
        WM_LOOK_DROP_ICON => delete_icon(),
        WM_LOOK_BACKDROP => backdrop(wp.0, lp),
        WM_LOOK_SHOT => {
            let tag = look_tag("shot");
            let r = match backdrop_rect() {
                Some(b) => capture::capture_union(&[b], 0, &shot_path(&format!("{tag}.png"))),
                None => Err("no backdrop".into()),
            };
            log!("shot '{tag}': {r:?}");
        }
        _ => {}
    }
}

/// While Haik has the menu open, save a screenshot each time the number of open menus settles.
fn on_watch_timer() {
    let wins = capture::menu_windows();
    let n = wins.len();
    let (seen, saved, count) = st(|s| {
        (
            s.watch_seen.replace(n),
            s.watch_saved.get(),
            s.live_captures.get(),
        )
    });
    if n == 0 || n != seen || n == saved || count >= MAX_LIVE_CAPTURES {
        return;
    }
    let theme = if theme::apps_light().unwrap_or(true) {
        "light"
    } else {
        "dark"
    };
    let name = format!("tray-menu-live-{theme}-{:02}-{n}menus.png", count + 1);
    let rects: Vec<RECT> = wins.iter().map(|w| w.1).collect();
    let r = capture::capture_union(&rects, 8, &shot_path(&name));
    log!("live capture: {n} menu window(s) -> {name}: {r:?}");
    st(|s| {
        s.watch_saved.set(n);
        s.live_captures.set(count + 1);
    });
}

fn on_command(cmd: u32) {
    match cmd {
        ID_AUTOMATIC => {
            st(|s| s.holding.set(!s.holding.get()));
            update_icon("Automatic toggled");
        }
        c if (ID_KEEP_BASE..ID_KEEP_BASE + KEEP_LABELS.len() as u32).contains(&c) => {
            st(|s| s.keep.set(c - ID_KEEP_BASE));
            log!("Keep awake: {}", KEEP_LABELS[(c - ID_KEEP_BASE) as usize]);
        }
        ID_SCREEN_ON => st(|s| s.screen_on.set(!s.screen_on.get())),
        ID_BALLOON => balloon(),
        ID_TOAST_REG => {
            register_toast_app();
            toast::show(
                toast::AUMID_REGISTERED,
                "Wakeshot spike",
                "Toast with a registered app id.",
            );
            mem("after toast");
        }
        ID_TOAST_UNREG => {
            toast::show(
                toast::AUMID_UNREGISTERED,
                "Wakeshot spike",
                "Toast with an unregistered app id.",
            );
            mem("after toast");
        }
        ID_SOUND_ALIAS => play_alias(),
        ID_SOUND_BUNDLED => {
            let ok = sound::play_bundled(st(|s| s.module.get()));
            log!("PlaySound bundled WAVE resource: {ok}");
        }
        ID_RESTORE => log!("Restore my power settings: the spike does nothing here"),
        ID_OPEN_LOG => unsafe {
            let p = HSTRING::from(log::path(st(|s| s.log_name.get())).as_os_str());
            ShellExecuteW(None, w!("open"), &p, None, None, SW_SHOWNORMAL);
        },
        ID_QUIT => quit(),
        _ => log!("command {cmd}"),
    }
}

fn play_alias() {
    log!(
        "sound scheme SystemNotification -> {:?}",
        sound::scheme_file("SystemNotification")
    );
    let ok = sound::play_alias("SystemNotification");
    log!("PlaySound alias SystemNotification: {ok}");
}

fn register_toast_app() {
    let icon = log::path("toast-icon.png");
    let r = capture::write_png(
        &icon,
        64,
        64,
        &glyph::glyph_rgba(64, true, [0x2b, 0x7c, 0xd3]),
    );
    log!("toast icon png: {r:?}");
    log!(
        "register HKCU AppUserModelId: {}",
        toast::register("Wakeshot tray spike", &icon.display().to_string())
    );
}

fn quit() {
    delete_icon();
    log!("unregister toast app ids: {}", toast::unregister());
    unsafe {
        let _ = DestroyWindow(st(|s| s.msg_hwnd.get()));
        let _ = DestroyWindow(hwnd());
    }
}

/// Saves what sits at the icon's rect. When the icon is in the overflow flyout, the rect is the `^` button.
fn capture_icon(tag: &str) {
    match icon_rect() {
        Ok(r) => {
            let theme = if theme::taskbar_light().unwrap_or(false) {
                "light"
            } else {
                "dark"
            };
            let name = format!("tray-icon-{tag}-{theme}-taskbar.png");
            let shot = capture::capture_union(&[r], 0, &shot_path(&name));
            log!("icon capture {r:?} -> {name}: {shot:?}");
        }
        Err(e) => log!("icon rect for capture: {e}"),
    }
}

// ---------- automatic run ----------

fn auto_step() {
    let mut step = st(|s| s.auto_step.replace(s.auto_step.get() + 1));
    if st(|s| s.mode.get()) == Mode::Menus && step > 5 {
        step = u32::MAX;
    }
    let next = |ms| timer(TIMER_AUTO, ms);
    kill_timer(TIMER_AUTO);
    match step {
        0 => {
            log!("Shell_NotifyIconGetRect after first add: {:?}", icon_rect());
            mem("idle after add");
            next(500);
        }
        1 => {
            st(|s| s.holding.set(false));
            update_icon("auto: idle glyph, idle tooltip");
            next(1000);
        }
        2 => {
            capture_icon("auto");
            st(|s| s.holding.set(true));
            update_icon("auto: holding glyph again");
            next(500);
        }
        3 => {
            log!(
                "SetPreferredAppMode(ForceLight): {:?}",
                ux().set_mode(AppMode::ForceLight)
            );
            ux().flush();
            capture_menu("forcelight", true);
            next(300);
        }
        4 => {
            log!(
                "SetPreferredAppMode(ForceDark): {:?}",
                ux().set_mode(AppMode::ForceDark)
            );
            ux().flush();
            capture_menu("forcedark", true);
            next(300);
        }
        5 => {
            log!(
                "SetPreferredAppMode(AllowDark): {:?}",
                ux().set_mode(AppMode::AllowDark)
            );
            ux().flush();
            let theme = if theme::apps_light().unwrap_or(true) {
                "light"
            } else {
                "dark"
            };
            capture_menu(&format!("allowdark-system-{theme}"), false);
            mem("after menus");
            next(300);
        }
        6 => {
            balloon();
            next(4000);
        }
        7 => {
            toast::show(
                toast::AUMID_UNREGISTERED,
                "Wakeshot spike",
                "Toast with an unregistered app id.",
            );
            next(3000);
        }
        8 => {
            log!(
                "history {}: {}",
                toast::AUMID_UNREGISTERED,
                toast::history_count(toast::AUMID_UNREGISTERED)
            );
            register_toast_app();
            toast::show(
                toast::AUMID_REGISTERED,
                "Wakeshot spike",
                "Toast with a registered app id.",
            );
            next(3000);
        }
        9 => {
            log!(
                "history {}: {}",
                toast::AUMID_REGISTERED,
                toast::history_count(toast::AUMID_REGISTERED)
            );
            mem("after toasts, WinRT loaded");
            play_alias();
            next(2000);
        }
        10 => {
            let ok = sound::play_bundled(st(|s| s.module.get()));
            log!("PlaySound bundled WAVE resource: {ok}");
            next(2000);
        }
        _ => {
            mem("end of auto run");
            quit();
        }
    }
}

// ---------- window procedure ----------

fn on_timer(id: usize) {
    match id {
        TIMER_AUTO => auto_step(),
        TIMER_MENU_CAPTURE => on_capture_timer(),
        TIMER_MENU_WATCH => on_watch_timer(),
        TIMER_THEME_SETTLE => {
            kill_timer(TIMER_THEME_SETTLE);
            let n = st(|s| s.switches.replace(s.switches.get() + 1)) + 1;
            let theme = if theme::apps_light().unwrap_or(true) {
                "light"
            } else {
                "dark"
            };
            log_theme("before switch captures");
            capture_icon(&format!("switch{n}"));
            capture_menu(&format!("switch{n}-{theme}-noflush"), false);
            ux().flush();
            capture_menu(&format!("switch{n}-{theme}-flush"), true);
        }
        TIMER_MEM => {
            kill_timer(TIMER_MEM);
            mem("idle");
            if let Mode::Idle(_) = st(|s| s.mode.get()) {
                quit();
            }
        }
        TIMER_ADD_RETRY => {
            if add_icon("retry") || st(|s| s.add_attempts.get()) >= 30 {
                kill_timer(TIMER_ADD_RETRY);
            }
        }
        TIMER_RECT => {
            kill_timer(TIMER_RECT);
            log!("Shell_NotifyIconGetRect: {:?}", icon_rect());
            let n = st(|s| s.add_attempts.get());
            capture_icon(&format!("add{n}"));
        }
        TIMER_END => {
            kill_timer(TIMER_END);
            guid_icon(NIM_DELETE);
            unsafe {
                let _ = DestroyWindow(st(|s| s.msg_hwnd.get()));
                let _ = DestroyWindow(hwnd());
            }
        }
        _ => {}
    }
}

fn on_tray(wp: WPARAM, lp: LPARAM) {
    // NOTIFYICON_VERSION_4: the event is in LOWORD(lParam), the anchor point in wParam.
    let event = (lp.0 as u32) & 0xffff;
    let x = (wp.0 & 0xffff) as i16 as i32;
    let y = ((wp.0 >> 16) & 0xffff) as i16 as i32;
    match event {
        WM_CONTEXTMENU => {
            log!("tray: right click at ({x}, {y})");
            st(|s| {
                s.watch_seen.set(0);
                s.watch_saved.set(0);
            });
            timer(TIMER_MENU_WATCH, 350);
            show_menu(x, y);
            kill_timer(TIMER_MENU_WATCH);
        }
        0x400 => log!("tray: NIN_SELECT (left click) at ({x}, {y})"),
        0x401 => log!("tray: NIN_KEYSELECT"),
        0x402 => log!("tray: NIN_BALLOONSHOW"),
        0x403 => log!("tray: NIN_BALLOONHIDE"),
        0x404 => log!("tray: NIN_BALLOONTIMEOUT"),
        0x405 => log!("tray: NIN_BALLOONUSERCLICK"),
        WM_MOUSEMOVE | 0x406 | 0x407 => {}
        other => log!("tray: event 0x{other:x}"),
    }
}

fn on_setting_change(main: bool, which: &str, wp: WPARAM, lp: LPARAM) {
    let text = if lp.0 != 0 {
        unsafe { PCWSTR(lp.0 as *const u16).to_string() }.unwrap_or_default()
    } else {
        String::new()
    };
    log!(
        "WM_SETTINGCHANGE on {which}: wParam=0x{:x} lParam='{text}'",
        wp.0
    );
    if wp.0 as u32 == SPI_SETHIGHCONTRAST.0 {
        log_theme("high contrast change");
        if main {
            update_icon("high contrast change");
        }
    }
    if text == "ImmersiveColorSet" && main {
        log_theme("ImmersiveColorSet");
        update_icon("theme change");
        if st(|s| s.mode.get()) == Mode::Interactive {
            // Windows sends several of these per switch; act once it settles.
            timer(TIMER_THEME_SETTLE, 2500);
        }
    }
}

extern "system" fn wndproc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let (main, msg_only, tc) = st(|s| (s.hwnd.get(), s.msg_hwnd.get(), s.taskbar_created.get()));
    let which = if h == main {
        "main window"
    } else if h == msg_only {
        "message-only window"
    } else {
        "window being created"
    };
    if tc != 0 && msg == tc {
        log!("TaskbarCreated received on {which}");
        if h == main {
            log_theme("TaskbarCreated");
            if !add_icon("TaskbarCreated") {
                timer(TIMER_ADD_RETRY, 2000);
            }
            timer(TIMER_RECT, 1500);
        }
        return LRESULT(0);
    }
    if h == main && (WM_LOOK_STYLE..=WM_LOOK_SHOT).contains(&msg) {
        on_look_message(msg, wp, lp);
        return LRESULT(0);
    }
    match msg {
        WM_MEASUREITEM => {
            let mis = unsafe { &mut *(lp.0 as *mut MEASUREITEMSTRUCT) };
            if mis.CtlType == ODT_MENU && look::measure(mis) {
                return LRESULT(1);
            }
            return unsafe { DefWindowProcW(h, msg, wp, lp) };
        }
        WM_DRAWITEM => {
            let dis = unsafe { &*(lp.0 as *const DRAWITEMSTRUCT) };
            if dis.CtlType == ODT_MENU && look::draw(dis) {
                return LRESULT(1);
            }
            return unsafe { DefWindowProcW(h, msg, wp, lp) };
        }
        WM_MENUCHAR => {
            if let Some(r) = look::menu_char(wp, lp) {
                return r;
            }
            return unsafe { DefWindowProcW(h, msg, wp, lp) };
        }
        WM_ENTERIDLE => {
            if wp.0 as u32 == MSGF_MENU {
                look::on_idle(HWND(lp.0 as *mut c_void));
            }
            return unsafe { DefWindowProcW(h, msg, wp, lp) };
        }
        WM_INITMENUPOPUP => {
            log!(
                "WM_INITMENUPOPUP menu 0x{:x} position {}",
                wp.0,
                lp.0 & 0xffff
            );
            return unsafe { DefWindowProcW(h, msg, wp, lp) };
        }
        // TPM_RETURNCMD hands the choice back to show_menu; this only records whether Windows also sends it.
        WM_COMMAND => log!("WM_COMMAND 0x{:x} (not acted on)", wp.0),
        WM_TRAY => on_tray(wp, lp),
        WM_TIMER => on_timer(wp.0),
        WM_SETTINGCHANGE => on_setting_change(h == main, which, wp, lp),
        WM_THEMECHANGED | WM_SYSCOLORCHANGE | WM_DPICHANGED | WM_DISPLAYCHANGE => {
            log!(
                "message 0x{msg:x} on {which} wParam=0x{:x}, system dpi now {}",
                wp.0,
                unsafe { GetDpiForSystem() }
            );
            if h == main && (msg == WM_DPICHANGED || msg == WM_DISPLAYCHANGE) {
                update_icon("display or dpi change");
            }
            return unsafe { DefWindowProcW(h, msg, wp, lp) };
        }
        // Lets a script stop the spike cleanly, with the icon removed.
        WM_CLOSE if h == main => quit(),
        WM_DESTROY if h == main => unsafe { PostQuitMessage(0) },
        _ => return unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
    LRESULT(0)
}
