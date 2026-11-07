//! Chime harness: plays Wakeshot's chimes through the real player, for a listening check.
//!
//! `ux_sounds [--cue lid|sleep|floor] [--all] [--wait]`
//!
//! `--cue` picks one chime (default `lid`); `--all` plays all three, 3 s apart; `--wait` uses `play_and_wait` (the
//! hibernate thread's call) instead of `play`. It prints the sound scheme, the endpoint the device rule picks, the
//! player's log lines and each result, each with the wall time. Exit code 0 when every chime played or was skipped.
//! It changes no setting: no volume, device or scheme.

use std::process::ExitCode;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use wakeshot_core::command::{Outcome, SoundEnd};
use wakeshot_core::event::Event;
use wakeshot_core::view::Cue;
use wakeshot_windows::WinError;
use wakeshot_windows::log::{Level, Log};
use wakeshot_windows::sink::EventSink;
use wakeshot_windows::sound::{
    SoundPlayer, WasapiPlayer, current_scheme, form_factor_name, picked_endpoint,
};
use windows::Win32::System::SystemInformation::GetLocalTime;

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

/// Hands each posted event to the main thread, as the loop window would get it.
struct ChannelSink(Mutex<Sender<Event>>);

impl EventSink for ChannelSink {
    fn post(&self, event: Event) -> Result<(), WinError> {
        // The main thread outlives every sound thread it waits for; a late one after exit has nobody to tell.
        let _ = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(event);
        Ok(())
    }
}

/// The player's log lines, printed as they come.
struct PrintLog;

impl Log for PrintLog {
    fn line(&self, level: Level, area: &'static str, text: &str) {
        say!("log {level:?} {area}: {text}");
    }
}

struct Args {
    cue: Cue,
    all: bool,
    wait: bool,
}

fn args() -> Result<Args, String> {
    let mut out = Args {
        cue: Cue::LidClosedHolding,
        all: false,
        wait: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--all" => out.all = true,
            "--wait" => out.wait = true,
            "--cue" => {
                out.cue = match it.next().as_deref() {
                    Some("lid") => Cue::LidClosedHolding,
                    Some("sleep") => Cue::GoingToSleep,
                    Some("floor") => Cue::BatteryFloor,
                    _ => return Err("--cue needs lid, sleep or floor".to_owned()),
                };
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(out)
}

fn main() -> ExitCode {
    let args = match args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\nusage: ux_sounds [--cue lid|sleep|floor] [--all] [--wait]");
            return ExitCode::from(2);
        }
    };
    say!(
        "sound scheme: {}",
        current_scheme().unwrap_or_else(|| "not readable".to_owned())
    );
    match picked_endpoint() {
        Ok((endpoint, why)) => say!(
            "picked endpoint: '{}', {} (form factor {:?}), {}",
            endpoint.name,
            form_factor_name(endpoint.form_factor),
            endpoint.form_factor,
            why.text()
        ),
        Err(e) => say!(
            "picked endpoint: none, {} failed with 0x{:08X}",
            e.call,
            e.code
        ),
    }

    let (tx, rx) = mpsc::channel();
    let player = WasapiPlayer::new(Arc::new(ChannelSink(Mutex::new(tx))), Arc::new(PrintLog));
    let cues = if args.all {
        vec![Cue::LidClosedHolding, Cue::GoingToSleep, Cue::BatteryFloor]
    } else {
        vec![args.cue]
    };
    let first = Instant::now();
    let mut all_good = true;
    for (i, cue) in (0u32..).zip(cues) {
        // Start to start, so the three are evenly spaced whatever each took.
        let due = first + Duration::from_secs(3) * i;
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
        let start = Instant::now();
        let result = if args.wait {
            say!("{cue:?}: play_and_wait");
            let result = player.play_and_wait(cue);
            say!(
                "{cue:?}: play_and_wait returned {result:?} after {} ms",
                start.elapsed().as_millis()
            );
            result.ok()
        } else {
            say!("{cue:?}: play");
            player.play(cue);
            say!(
                "{cue:?}: play returned after {} us",
                start.elapsed().as_micros()
            );
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(Event::Done(Outcome::Sound(result))) => {
                    say!(
                        "{cue:?}: Done(Sound({result:?})) after {} ms",
                        start.elapsed().as_millis()
                    );
                    result.ok()
                }
                Ok(other) => {
                    say!("{cue:?}: unexpected event {other:?}");
                    None
                }
                Err(_) => {
                    say!("{cue:?}: no result within 5 s");
                    None
                }
            }
        };
        all_good &= matches!(result, Some(SoundEnd::Played | SoundEnd::Skipped));
    }
    if all_good {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
