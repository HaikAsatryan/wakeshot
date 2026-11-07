//! Tray icon harness: the demo icon on the real taskbar, for pictures and click checks.
//!
//! `ux_icon [--state holding|idle|off] [--cycle] [--pattern] [--contrast] [--minutes N]`
//!
//! Start it at Medium, as a user's tray runs (`explorer.exe <launch .cmd>` from an elevated shell). It prints each
//! event with the wall time, and quits after N minutes (default 10) or on `WM_CLOSE` (`taskkill /PID <pid>` without
//! `/F`), removing its icon either way.

use wakeshot_core::view::{IconState, MenuCommand, MenuItem, MenuModel, TrayView};
use wakeshot_windows::tray::icon::{IconKind, TrayIcon};
use wakeshot_windows::tray::theme::{Contrast, Rgb, Theme};
use wakeshot_windows::tray::{Anchor, MenuView};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, SetTimer, TranslateMessage, WM_CLOSE, WM_TIMER,
};

fn now() -> String {
    // SAFETY: plain call.
    let t = unsafe { GetLocalTime() };
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

macro_rules! say {
    ($($arg:tt)*) => { println!("{} {}", now(), format!($($arg)*)) };
}

/// Prints where the menu would open, so each kind of click can be checked against the icon's place.
struct Printer;

impl MenuView for Printer {
    fn show(&mut self, _menu: &MenuModel, anchor: Anchor) {
        say!("menu: anchor ({}, {})", anchor.x, anchor.y);
    }
}

struct Args {
    state: IconState,
    cycle: bool,
    pattern: bool,
    contrast: bool,
    minutes: u32,
}

fn args() -> Result<Args, String> {
    let mut out = Args {
        state: IconState::Holding,
        cycle: false,
        pattern: false,
        contrast: false,
        minutes: 10,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--cycle" => out.cycle = true,
            "--pattern" => out.pattern = true,
            "--contrast" => out.contrast = true,
            "--minutes" => {
                out.minutes = it
                    .next()
                    .and_then(|n| n.parse().ok())
                    .ok_or("--minutes needs a whole number")?;
            }
            "--state" => {
                out.state = match it.next().as_deref() {
                    Some("holding") => IconState::Holding,
                    Some("idle") => IconState::Idle,
                    Some("off") => IconState::Off,
                    _ => return Err("--state needs holding, idle or off".to_owned()),
                };
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(out)
}

fn view(state: IconState) -> TrayView {
    let line = match state {
        IconState::Holding => "Holding awake: Claude Code, 2 sessions",
        IconState::Idle => "Idle: the PC sleeps as usual",
        IconState::Off => "Automatic is off",
    };
    TrayView {
        icon: state,
        tooltip: format!("Wakeshot demo\n{line}"),
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

/// A sample high contrast pair: yellow text on black.
fn contrast_theme() -> Theme {
    let black = Rgb::new(0, 0, 0);
    let yellow = Rgb::new(0xff, 0xff, 0);
    Theme {
        light: false,
        contrast: Some(Contrast {
            window: black,
            window_text: yellow,
            highlight: Rgb::new(0x1a, 0xeb, 0xff),
            highlight_text: black,
            gray_text: Rgb::new(0x3f, 0xf2, 0x3f),
            button_face: black,
            button_text: yellow,
            hot_light: Rgb::new(0x80, 0x80, 0xff),
        }),
    }
}

fn next(state: IconState) -> IconState {
    match state {
        IconState::Holding => IconState::Idle,
        IconState::Idle => IconState::Off,
        IconState::Off => IconState::Holding,
    }
}

fn main() {
    let args = match args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    // The product exe gets this from its manifest; without it the taskbar size and the anchors would be scaled.
    if let Err(e) =
        // SAFETY: plain call, before any window exists.
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
    {
        say!("SetProcessDpiAwarenessContext failed: {e}");
    }
    let mut icon = match TrayIcon::new(IconKind::Demo, Box::new(Printer)) {
        Ok(icon) => icon,
        Err(e) => {
            eprintln!("TrayIcon::new failed: {e:?}");
            std::process::exit(1);
        }
    };
    icon.on_note(Box::new(|note| say!("icon: {note:?}")));
    icon.on_balloon_click(Box::new(|| say!("balloon click")));
    let id = icon.identity();
    say!(
        "pid {} window {:#x} guid {:032x}",
        std::process::id(),
        id.hwnd,
        id.guid
    );
    if args.contrast {
        icon.force_theme(Some(contrast_theme()));
    }
    if args.pattern {
        icon.show_pattern(true);
    }
    let mut state = args.state;
    icon.show(&view(state));
    say!("shown: {state:?}");

    // Thread timers: their WM_TIMER comes with no window and is read here.
    // SAFETY: plain calls; no window and no timer procedure.
    let end = unsafe { SetTimer(None, 0, args.minutes.saturating_mul(60_000), None) };
    let cycle = if args.cycle {
        // SAFETY: as above.
        unsafe { SetTimer(None, 0, 3000, None) }
    } else {
        0
    };
    let mut msg = MSG::default();
    // SAFETY: `msg` is a valid out pointer.
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
        if msg.message == WM_CLOSE {
            say!("WM_CLOSE: quitting");
            break;
        }
        if msg.message == WM_TIMER && msg.hwnd.is_invalid() {
            if msg.wParam.0 == end {
                say!("time is up");
                break;
            }
            if cycle != 0 && msg.wParam.0 == cycle {
                state = next(state);
                icon.show(&view(state));
                say!("shown: {state:?}");
            }
            continue;
        }
        // SAFETY: `msg` came from `GetMessageW` just above.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    drop(icon);
    say!("icon removed");
}
