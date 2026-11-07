//! Tray menu harness: the demo icon with the real owner-drawn menu, for pictures, keyboard and screen reader checks.
//!
//! `ux_menu [--contrast] [--minutes N]`
//!
//! Start it at Medium, as a user's tray runs (`explorer.exe <launch .cmd>` from an elevated shell). It prints each
//! event and each menu note with the wall time, and quits after N minutes (default 10) or on `WM_CLOSE`
//! (`taskkill /PID <pid>` without `/F`), removing its icon either way. A pick is only printed: the menu shows the same
//! sample view every time, as the core would decide what a pick does.

use std::sync::Arc;

use wakeshot_core::event::Event;
use wakeshot_core::holds::HoldKind;
use wakeshot_core::text::{self, Status};
use wakeshot_core::view::{IconState, KeepAwake, MenuCommand, MenuItem, MenuModel, TrayView};
use wakeshot_windows::WinError;
use wakeshot_windows::sink::EventSink;
use wakeshot_windows::tray::icon::{IconKind, TrayIcon};
use wakeshot_windows::tray::menu::OwnerDrawnMenu;
use wakeshot_windows::tray::theme::{Contrast, Rgb, Theme};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, SetTimer, TranslateMessage, WM_CLOSE, WM_TIMER,
};

/// A hold reason in Armenian, as escapes so the source stays ASCII: "Nightly backup & whole disk check". Long, so the
/// status line ends in an ellipsis; with a literal `&`, so a screen reader check shows whether it survives.
const ARMENIAN_REASON: &str = "\u{533}\u{56b}\u{577}\u{565}\u{580}\u{561}\u{575}\u{56b}\u{576} \
    \u{57a}\u{561}\u{570}\u{578}\u{582}\u{57d}\u{57f}\u{561}\u{57e}\u{578}\u{580}\u{578}\u{582}\u{574} & \
    \u{561}\u{574}\u{562}\u{578}\u{572}\u{57b} \u{57d}\u{56f}\u{561}\u{57e}\u{561}\u{57c}\u{561}\u{56f}\u{56b} \
    \u{57d}\u{57f}\u{578}\u{582}\u{563}\u{578}\u{582}\u{574}";

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

/// Stands in for the loop's sink: prints each event the menu posts.
struct Printer;

impl EventSink for Printer {
    fn post(&self, event: Event) -> Result<(), WinError> {
        say!("event: {event:?}");
        Ok(())
    }
}

struct Args {
    contrast: bool,
    minutes: u32,
}

fn args() -> Result<Args, String> {
    let mut out = Args {
        contrast: false,
        minutes: 10,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--contrast" => out.contrast = true,
            "--minutes" => {
                out.minutes = it
                    .next()
                    .and_then(|n| n.parse().ok())
                    .ok_or("--minutes needs a whole number")?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(out)
}

fn action(command: MenuCommand, checked: bool, enabled: bool) -> MenuItem {
    MenuItem::Action {
        text: text::action(command),
        command,
        checked,
        enabled,
    }
}

/// The product sketch as `present` builds it: holding for two Claude Code sessions and a script with a long Armenian
/// reason, Automatic on, Keep awake off, so the menu shows a check, a bullet, a disabled item and a submenu.
fn view() -> TrayView {
    let status = Status {
        icon: IconState::Holding,
        stepped_back: false,
        requests_only: false,
        screen_on: false,
    };
    let groups = [
        text::hold_group(HoldKind::Agent, "Claude Code", 2),
        text::hold_group(HoldKind::Script, ARMENIAN_REASON, 1),
    ];
    let mut keep_awake: Vec<MenuItem> = [
        KeepAwake::Off,
        KeepAwake::Minutes(30),
        KeepAwake::Minutes(60),
        KeepAwake::Minutes(120),
        KeepAwake::Minutes(240),
        KeepAwake::UntilOff,
    ]
    .into_iter()
    .map(|k| action(MenuCommand::KeepAwake(k), k == KeepAwake::Off, true))
    .collect();
    keep_awake.push(MenuItem::Separator);
    keep_awake.push(action(MenuCommand::KeepScreenOn(true), false, false));
    TrayView {
        icon: IconState::Holding,
        tooltip: text::tooltip(&status, &groups),
        menu: MenuModel {
            items: vec![
                MenuItem::Label {
                    text: text::status_line(&status, &groups),
                    icon: Some(status.icon),
                },
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
            ],
        },
    }
}

/// The sample high contrast set `ux_icon` uses: yellow text on black.
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

fn main() {
    let args = match args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    // The product exe gets this from its manifest; without it the menu's DPI and the anchors would be scaled.
    // SAFETY: plain call, before any window exists.
    if let Err(e) =
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
    {
        say!("SetProcessDpiAwarenessContext failed: {e}");
    }
    let mut menu = match OwnerDrawnMenu::new(Arc::new(Printer)) {
        Ok(menu) => menu,
        Err(e) => {
            eprintln!("OwnerDrawnMenu::new failed: {e:?}");
            std::process::exit(1);
        }
    };
    menu.on_note(Box::new(|note| say!("menu: {note:?}")));
    if args.contrast {
        menu.force_theme(Some(contrast_theme()));
    }
    let mut icon = match TrayIcon::new(IconKind::Demo, Box::new(menu)) {
        Ok(icon) => icon,
        Err(e) => {
            eprintln!("TrayIcon::new failed: {e:?}");
            std::process::exit(1);
        }
    };
    icon.on_note(Box::new(|note| say!("icon: {note:?}")));
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
    let view = view();
    say!("status line: {:?}", view.menu.items[0]);
    icon.show(&view);

    // A thread timer: its WM_TIMER comes with no window and is read here.
    // SAFETY: plain call; no window and no timer procedure.
    let end = unsafe { SetTimer(None, 0, args.minutes.saturating_mul(60_000), None) };
    let mut msg = MSG::default();
    // SAFETY: `msg` is a valid out pointer.
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
        if msg.message == WM_CLOSE {
            say!("WM_CLOSE: quitting");
            break;
        }
        if msg.message == WM_TIMER && msg.hwnd.is_invalid() && msg.wParam.0 == end {
            say!("time is up");
            break;
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
