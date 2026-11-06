//! Notices harness (`p5-notices` Part B): the demo icon plus `BalloonNotifier`, for banner pictures and click checks.
//!
//! `ux_notices [--all | --one <name> | --pair | --list] [--icon user|info] [--cycle] [--minutes N]`
//!
//! - `--all`: every notice with sample data, 10 s apart (the default).
//! - `--one <name>`: one notice; `--list` prints the names and quits.
//! - `--pair`: the first-run notice, then the failed restore 1 s later (two different click actions).
//! - `--icon user|info`: the large icon of a notice that is not a warning: the app icon or Windows' info icon.
//! - `--cycle`: the icon changes state every 3 s while the notices show, so the loop thread's icon changes and the
//!   notice thread's balloons run side by side.
//!
//! Start it at Medium (Phase 5 track note 4). As in the product, the notices come from a thread of their own and the
//! icon lives on the main thread. A balloon click prints the action `take_click` gives; nothing is opened. It quits
//! after N minutes (default 10) or on `WM_CLOSE` (`taskkill /PID <pid>` without `/F`), removing its icon either way.

use std::sync::Arc;
use std::time::Duration;

use wakeshot_core::settings::SettingKey;
use wakeshot_core::text;
use wakeshot_core::view::{IconState, MenuCommand, MenuItem, MenuModel, Notice, TrayView};
use wakeshot_windows::notify::{BalloonNotifier, IconChoice, Notifier};
use wakeshot_windows::tray::icon::{IconKind, TrayIcon};
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

/// Every click opens nothing here; the menu would only get in the way of the banners.
struct NoMenu;

impl MenuView for NoMenu {
    fn show(&mut self, _menu: &MenuModel, anchor: Anchor) {
        say!("menu: anchor ({}, {}), not opened", anchor.x, anchor.y);
    }
}

enum Run {
    All,
    One(Notice),
    Pair,
}

struct Args {
    run: Run,
    icon: IconChoice,
    cycle: bool,
    minutes: u32,
}

/// The name `--one` takes. No wildcard: a new notice does not compile here until it has a name and a sample.
fn name(notice: &Notice) -> &'static str {
    match notice {
        Notice::FirstRun => "first-run",
        Notice::UserChanged(_) => "user-changed",
        Notice::RestoredByUser => "restored-by-user",
        Notice::StepBack { .. } => "step-back",
        Notice::RequestsOnly => "requests-only",
        Notice::CannotHold => "cannot-hold",
        Notice::RestoreFailed => "restore-failed",
        Notice::HibernateFailed => "hibernate-failed",
        Notice::LockFailed => "lock-failed",
        Notice::ClosedLidReport { .. } => "closed-lid-report",
        Notice::UpdateWakeshot { .. } => "update-wakeshot",
        Notice::JournalLost { .. } => "journal-lost",
        Notice::IdleLocked => "idle-locked",
        Notice::Updated { .. } => "updated",
        Notice::HooksAdded { .. } => "hooks-added",
    }
}

/// One of each, with the sample data `text.rs` tests with.
fn samples() -> Vec<Notice> {
    vec![
        Notice::FirstRun,
        Notice::UserChanged(SettingKey::LidAc),
        Notice::RestoredByUser,
        Notice::StepBack { floor_pct: 20 },
        Notice::RequestsOnly,
        Notice::CannotHold,
        Notice::RestoreFailed,
        Notice::HibernateFailed,
        Notice::LockFailed,
        Notice::ClosedLidReport {
            ran_ms: 135 * 60_000,
            held_by: vec!["Claude Code".to_owned(), "cargo".to_owned()],
        },
        Notice::UpdateWakeshot {
            agent: "Claude Code",
        },
        Notice::JournalLost {
            reset: vec![SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc],
            failed: false,
        },
        Notice::IdleLocked,
        Notice::Updated {
            version: "1.1.0".to_owned(),
        },
        Notice::HooksAdded { agent: "Cursor" },
    ]
}

fn args() -> Result<Args, String> {
    let mut out = Args {
        run: Run::All,
        icon: IconChoice::PRODUCT,
        cycle: false,
        minutes: 10,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--all" => out.run = Run::All,
            "--pair" => out.run = Run::Pair,
            "--cycle" => out.cycle = true,
            "--list" => {
                for notice in samples() {
                    println!("{}", name(&notice));
                }
                std::process::exit(0);
            }
            "--one" => {
                let wanted = it.next().ok_or("--one needs a name (see --list)")?;
                let notice = samples()
                    .into_iter()
                    .find(|n| name(n) == wanted)
                    .ok_or(format!("no notice named {wanted} (see --list)"))?;
                out.run = Run::One(notice);
            }
            "--icon" => {
                out.icon = match it.next().as_deref() {
                    Some("user") => IconChoice::App,
                    Some("info") => IconChoice::Info,
                    _ => return Err("--icon needs user or info".to_owned()),
                };
            }
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

fn view(state: IconState) -> TrayView {
    let line = match state {
        IconState::Holding => "Holding awake: Claude Code (2 sessions)",
        IconState::Idle => "Not holding, the PC sleeps as usual",
        IconState::Off => "Automatic is off, the PC sleeps as usual",
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

fn next(state: IconState) -> IconState {
    match state {
        IconState::Holding => IconState::Idle,
        IconState::Idle => IconState::Off,
        IconState::Off => IconState::Holding,
    }
}

/// The notices and the wait before each one. The first waits 2 s, so the icon is on the taskbar first.
fn plan(run: Run) -> Vec<(Duration, Notice)> {
    let first = Duration::from_secs(2);
    let (list, gap) = match run {
        Run::All => (samples(), Duration::from_secs(10)),
        Run::One(notice) => (vec![notice], Duration::ZERO),
        Run::Pair => (
            vec![Notice::FirstRun, Notice::RestoreFailed],
            Duration::from_secs(1),
        ),
    };
    list.into_iter()
        .enumerate()
        .map(|(i, n)| (if i == 0 { first } else { gap }, n))
        .collect()
}

fn main() {
    let args = match args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    // The product exe gets this from its manifest; the banner's icon size follows the taskbar's DPI either way.
    // SAFETY: plain call, before any window exists.
    if let Err(e) =
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
    {
        say!("SetProcessDpiAwarenessContext failed: {e}");
    }
    let mut icon = match TrayIcon::new(IconKind::Demo, Box::new(NoMenu)) {
        Ok(icon) => icon,
        Err(e) => {
            eprintln!("TrayIcon::new failed: {e:?}");
            std::process::exit(1);
        }
    };
    icon.on_note(Box::new(|note| say!("icon: {note:?}")));
    let id = icon.identity();
    let notifier = Arc::new(BalloonNotifier::with_choice(id, args.icon));
    let clicks = Arc::clone(&notifier);
    icon.on_balloon_click(Box::new(move || {
        say!("balloon click: {:?}", clicks.take_click());
    }));
    say!(
        "pid {} window {:#x} guid {:032x} icon {:?}",
        std::process::id(),
        id.hwnd,
        id.guid,
        args.icon
    );
    let mut state = IconState::Holding;
    icon.show(&view(state));
    say!("shown: {state:?}");

    // The IO thread's part: it owns a `Box<dyn Notifier>`, as `Command::Notify` does in the product.
    let plan = plan(args.run);
    let shower: Box<dyn Notifier> = Box::new(Arc::clone(&notifier));
    std::thread::spawn(move || {
        for (wait, notice) in plan {
            std::thread::sleep(wait);
            let t = text::notice(&notice);
            say!(
                "notice {}: warning {} silent {} click {:?}",
                name(&notice),
                t.warning,
                t.silent,
                t.click
            );
            say!("notice {}: {:?}", name(&notice), shower.show(&notice));
        }
        say!("all notices sent");
    });

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
