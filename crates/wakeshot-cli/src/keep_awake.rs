//! `wakeshot keep-awake`: Keep awake from a terminal, through the tray's hold file.
//!
//! Owner: sources track (`p4-cli-commands`); fixes by `p6-int-dry-run-e2e`.

use std::process::ExitCode;

use wakeshot_core::files::StateFile;
use wakeshot_core::holds::file::{format_expires, parse_expires};
use wakeshot_core::sources::cron::LocalOffset;
use wakeshot_core::text;
use wakeshot_core::view::{KeepAwake, MenuCommand};
use wakeshot_windows::clock;
use wakeshot_windows::localtime::SystemZone;
use wakeshot_windows::log::{Level, Log};
use wakeshot_windows::process::{Liveness, SystemLiveness};
use wakeshot_windows::store::{self, DataDir, DataFile};

use crate::command::Global;
use crate::status::{self, AREA, Answer, Tray};

const NOT_RUNNING: &str = "Wakeshot is not running. Start it, then try again.";
const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 24 * 60 * MINUTE_MS;

pub fn main(global: &Global, choice: KeepAwake, screen: bool) -> ExitCode {
    let (dir, log) = match status::open(global) {
        Ok(opened) => opened,
        Err(answer) => return answer.print(),
    };
    let now = clock::now().wall_ms;
    let zone = SystemZone::current();
    run(&dir, &SystemLiveness, &*log, now, &zone, choice, screen).print()
}

/// `now_ms`: wall time, the clock a Keep awake end is read on; `zone`: for the end in local time.
pub(crate) fn run(
    dir: &DataDir,
    liveness: &dyn Liveness,
    log: &dyn Log,
    now_ms: i64,
    zone: &dyn LocalOffset,
    choice: KeepAwake,
    screen: bool,
) -> Answer {
    if choice == KeepAwake::Off {
        return off(dir, log);
    }
    let tray = status::tray(dir, liveness);
    let state = match tray {
        Tray::Running(state) => state,
        other => {
            let text = format!("keep-awake: not set, {}", other.describe());
            log.line(other.level(), AREA, &text);
            return Answer::failed(match other {
                Tray::Newer(_) => status::NEWER,
                Tray::Unknown => status::UNKNOWN,
                Tray::Unreadable(_) => status::UNREADABLE,
                Tray::NotRunning | Tray::Running(_) => NOT_RUNNING,
            });
        }
    };
    let expires = match choice {
        KeepAwake::Minutes(minutes) => Some(format_expires(
            now_ms.saturating_add(i64::from(minutes) * MINUTE_MS),
        )),
        KeepAwake::Off | KeepAwake::UntilOff => None,
    };
    let bytes = hold_json(&state, expires.as_deref(), screen);
    if let Err(e) = store::write_atomic(&dir.file(DataFile::KeepAwake), bytes.as_bytes()) {
        let text = format!(
            "keep-awake: hold file not written, {}",
            status::error_text(e)
        );
        log.line(Level::Warn, AREA, &text);
        return Answer::failed(format!(
            "Could not turn on Keep awake: {}.",
            status::error_text(e)
        ));
    }
    let until = expires.as_deref().unwrap_or("turned off");
    let text = format!(
        "keep-awake: set until {until}, screen {}, tray pid {}",
        on_off(screen),
        state.pid
    );
    log.line(Level::Info, AREA, &text);
    // The end as written, so the time shown is the one the tray reads back (whole seconds).
    let end = expires.as_deref().and_then(parse_expires);
    Answer::ok(said(choice, end, now_ms, zone, screen))
}

fn off(dir: &DataDir, log: &dyn Log) -> Answer {
    // Works whether or not the tray runs: the file names the tray's own process, so a stale one holds nothing, and
    // Off is what the user asked for either way.
    match store::remove(&dir.file(DataFile::KeepAwake)) {
        Ok(existed) => {
            let what = match existed {
                true => "hold file deleted",
                false => "there was no hold file",
            };
            log.line(Level::Info, AREA, &format!("keep-awake off: {what}"));
            Answer::ok("Keep awake is off.")
        }
        Err(e) => {
            let text = format!("keep-awake off: not deleted, {}", status::error_text(e));
            log.line(Level::Warn, AREA, &text);
            Answer::failed(format!(
                "Could not turn off Keep awake: {}.",
                status::error_text(e)
            ))
        }
    }
}

/// The hold file in the form `HoldFile` reads (architecture section 6), with the tray's pid and start, so Keep awake
/// ends with the tray. Written by hand: every value is a number, a bool or `format_expires` text, so nothing needs
/// escaping, and this crate has no JSON writer.
fn hold_json(tray: &StateFile, expires: Option<&str>, screen: bool) -> String {
    let expires = expires
        .map(|e| format!(r#","expires":"{e}""#))
        .unwrap_or_default();
    format!(
        r#"{{"version":1,"pid":{},"procStart":"{}"{expires},"screen":{screen}}}"#,
        tray.pid, tray.proc_start.0
    )
}

fn said(
    choice: KeepAwake,
    end: Option<i64>,
    now_ms: i64,
    zone: &dyn LocalOffset,
    screen: bool,
) -> String {
    let what = match (choice, end) {
        (KeepAwake::Minutes(minutes), Some(end)) => format!(
            "Keep awake is on for {}, until {}",
            length(minutes),
            local_time(end, now_ms, zone)
        ),
        _ => "Keep awake is on until you turn it off".to_owned(),
    };
    match screen {
        true => format!("{what}, and the screen stays on."),
        false => format!("{what}."),
    }
}

/// The menu's own words for a length ("30 minutes", "1 hour"), without the access key mark.
fn length(minutes: u32) -> String {
    let menu = text::action(MenuCommand::KeepAwake(KeepAwake::Minutes(minutes)));
    without_access_keys(&menu)
}

/// The Win32 menu rule: `&` marks the access key, `&&` shows one `&`.
fn without_access_keys(menu: &str) -> String {
    let mut out = String::with_capacity(menu.len());
    let mut chars = menu.chars();
    while let Some(c) = chars.next() {
        match c {
            '&' => out.extend(chars.next()),
            c => out.push(c),
        }
    }
    out
}

/// "01:15", "01:15 tomorrow", or the date for anything later, in the PC's zone at that time.
fn local_time(utc_ms: i64, now_ms: i64, zone: &dyn LocalOffset) -> String {
    let local = |ms: i64| {
        let shifted = ms.saturating_add(i64::from(zone.offset_minutes(ms)) * MINUTE_MS);
        // `format_expires` gives `YYYY-MM-DDTHH:MM:SSZ`; on a shifted time its digits are the local date and time.
        let text = format_expires(shifted);
        (text[..10].to_owned(), text[11..16].to_owned())
    };
    let (date, time) = local(utc_ms);
    if date == local(now_ms).0 {
        time
    } else if date == local(now_ms.saturating_add(DAY_MS)).0 {
        format!("{time} tomorrow")
    } else {
        format!("{date} {time}")
    }
}

fn on_off(on: bool) -> &'static str {
    match on {
        true => "on",
        false => "off",
    }
}

#[cfg(test)]
mod tests {
    use wakeshot_core::files::{self, HoldFile, Read};
    use wakeshot_core::process::Alive;
    use wakeshot_core::sources::cron::FixedOffset;
    use wakeshot_windows::log::MemoryLog;
    use wakeshot_windows::process::FakeLiveness;

    use super::*;
    use crate::status::fixture::{TempData, dead, me, state_json};

    /// 2026-11-03T20:00:00Z: 00:00 the next day in Yerevan (UTC+4).
    const NOW: i64 = 1_793_736_000_000;
    const YEREVAN: FixedOffset = FixedOffset(240);

    fn running(tag: &str) -> TempData {
        let data = TempData::new(tag);
        data.write(DataFile::State, &state_json(me(), true, &[], &[], false));
        data
    }

    fn keep_awake(data: &TempData, choice: KeepAwake, screen: bool) -> (Answer, Vec<String>) {
        let log = MemoryLog::default();
        let answer = run(
            &data.dir,
            &SystemLiveness,
            &log,
            NOW,
            &YEREVAN,
            choice,
            screen,
        );
        let lines = log.lines.into_inner().unwrap();
        (answer, lines.into_iter().map(|(_, _, text)| text).collect())
    }

    fn hold(data: &TempData) -> HoldFile {
        let bytes = data.read(DataFile::KeepAwake).expect("keep-awake.json");
        match files::read(&bytes) {
            Read::Value(hold) => hold,
            other => panic!("{other:?}: {}", String::from_utf8_lossy(&bytes)),
        }
    }

    #[test]
    fn each_length_writes_the_trays_hold_with_its_end() {
        let tray = me();
        for (minutes, said) in [
            (30, "Keep awake is on for 30 minutes, until 00:30."),
            (60, "Keep awake is on for 1 hour, until 01:00."),
            (120, "Keep awake is on for 2 hours, until 02:00."),
            (240, "Keep awake is on for 4 hours, until 04:00."),
        ] {
            let data = running("ka-minutes");
            let (answer, log) = keep_awake(&data, KeepAwake::Minutes(minutes), false);
            assert_eq!(answer, Answer::ok(said));
            let hold = hold(&data);
            assert_eq!(
                (hold.pid, hold.proc_start, hold.screen, hold.reason),
                (tray.pid, Some(tray.start), false, None)
            );
            let end = parse_expires(hold.expires.as_deref().unwrap()).unwrap();
            let want = NOW + i64::from(minutes) * MINUTE_MS;
            assert!((want - end).abs() < 1000, "{end} vs {want}");
            assert_eq!(
                log,
                [format!(
                    "keep-awake: set until {}, screen off, tray pid {}",
                    format_expires(want),
                    tray.pid
                )]
            );
        }
    }

    /// With the real clock, as `main` runs it: the end lands within 1 s of now plus the minutes.
    #[test]
    fn end_is_now_plus_the_minutes_on_the_wall_clock() {
        let data = running("ka-clock");
        let before = clock::now().wall_ms;
        let answer = run(
            &data.dir,
            &SystemLiveness,
            &MemoryLog::default(),
            clock::now().wall_ms,
            &SystemZone::current(),
            KeepAwake::Minutes(60),
            false,
        );
        assert_eq!(answer.code, 0, "{answer:?}");
        let end = parse_expires(hold(&data).expires.as_deref().unwrap()).unwrap();
        let want = before + 60 * MINUTE_MS;
        assert!((end - want).abs() <= 1000, "{end} vs {want}");
    }

    #[test]
    fn until_off_has_no_end_and_keeps_the_screen_flag() {
        let data = running("ka-on");
        let (answer, _) = keep_awake(&data, KeepAwake::UntilOff, true);
        assert_eq!(
            answer,
            Answer::ok("Keep awake is on until you turn it off, and the screen stays on.")
        );
        let hold = hold(&data);
        assert_eq!((hold.expires, hold.screen), (None, true));
    }

    #[test]
    fn screen_on_with_a_length() {
        let data = running("ka-screen");
        let (answer, _) = keep_awake(&data, KeepAwake::Minutes(30), true);
        assert_eq!(
            answer,
            Answer::ok("Keep awake is on for 30 minutes, until 00:30, and the screen stays on.")
        );
        assert!(hold(&data).screen);
    }

    /// A new choice replaces the old file whole, as the menu does.
    #[test]
    fn a_new_choice_replaces_the_old_one() {
        let data = running("ka-replace");
        keep_awake(&data, KeepAwake::Minutes(240), true);
        keep_awake(&data, KeepAwake::UntilOff, false);
        let hold = hold(&data);
        assert_eq!((hold.expires, hold.screen), (None, false));
    }

    #[test]
    fn off_deletes_the_file_and_a_missing_one_is_fine() {
        let data = running("ka-off");
        keep_awake(&data, KeepAwake::UntilOff, false);
        let (answer, log) = keep_awake(&data, KeepAwake::Off, false);
        assert_eq!(answer, Answer::ok("Keep awake is off."));
        assert_eq!(log, ["keep-awake off: hold file deleted"]);
        assert_eq!(data.read(DataFile::KeepAwake), None);
        let (again, log) = keep_awake(&data, KeepAwake::Off, false);
        assert_eq!(again, Answer::ok("Keep awake is off."));
        assert_eq!(log, ["keep-awake off: there was no hold file"]);
    }

    /// Off needs no tray: a file left by a tray that ended holds nothing, and it goes.
    #[test]
    fn off_works_without_the_tray() {
        let data = TempData::new("ka-off-dead");
        data.write(DataFile::KeepAwake, r#"{"version":1,"pid":1}"#);
        let (answer, _) = keep_awake(&data, KeepAwake::Off, false);
        assert_eq!(answer, Answer::ok("Keep awake is off."));
        assert_eq!(data.read(DataFile::KeepAwake), None);
    }

    #[test]
    fn needs_the_tray_running() {
        let none = TempData::new("ka-none");
        let gone = TempData::new("ka-gone");
        gone.write(DataFile::State, &state_json(dead(), true, &[], &[], false));
        for data in [&none, &gone] {
            let (answer, log) = keep_awake(data, KeepAwake::Minutes(60), false);
            assert_eq!(answer, Answer::failed(NOT_RUNNING));
            assert_eq!(log, ["keep-awake: not set, not running"]);
            assert_eq!(data.read(DataFile::KeepAwake), None);
        }
    }

    #[test]
    fn newer_or_unknown_state_writes_nothing() {
        let newer = TempData::new("ka-newer");
        newer.write(DataFile::State, r#"{ "version": 2 }"#);
        assert_eq!(
            keep_awake(&newer, KeepAwake::UntilOff, false).0,
            Answer::failed(status::NEWER)
        );
        assert_eq!(newer.read(DataFile::KeepAwake), None);

        let unknown = TempData::new("ka-unknown");
        let tray = me();
        unknown.write(DataFile::State, &state_json(tray, true, &[], &[], false));
        let liveness = FakeLiveness::default();
        liveness.alive.lock().unwrap().push((tray, Alive::Unknown));
        let answer = run(
            &unknown.dir,
            &liveness,
            &MemoryLog::default(),
            NOW,
            &YEREVAN,
            KeepAwake::UntilOff,
            false,
        );
        assert_eq!(answer, Answer::failed(status::UNKNOWN));
        assert_eq!(unknown.read(DataFile::KeepAwake), None);
    }

    #[test]
    fn end_reads_as_local_time() {
        let hour = 60 * MINUTE_MS;
        // 23:30 local; Yerevan is UTC+4.
        let now = NOW - 30 * MINUTE_MS;
        assert_eq!(local_time(now + 20 * MINUTE_MS, now, &YEREVAN), "23:50");
        assert_eq!(local_time(now + hour, now, &YEREVAN), "00:30 tomorrow");
        assert_eq!(
            local_time(now + 2 * DAY_MS, now, &YEREVAN),
            "2026-11-05 23:30"
        );
        assert_eq!(local_time(NOW, NOW, &FixedOffset(0)), "20:00");
        assert_eq!(local_time(NOW, NOW, &FixedOffset(-330)), "14:30");
    }

    #[test]
    fn lengths_are_the_menus_words() {
        assert_eq!(length(30), "30 minutes");
        assert_eq!(length(60), "1 hour");
        assert_eq!(length(90), "1 hour 30 minutes");
        assert_eq!(without_access_keys("&Fish && chips"), "Fish & chips");
    }
}
