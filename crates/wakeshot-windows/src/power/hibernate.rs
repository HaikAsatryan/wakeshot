//! The hibernate thread: the last check, the going to sleep sound, the lid once more, `SetSuspendState`. It also runs
//! the sleep check: the same check and sound with no hibernate, before the core sets the 1-minute sleep and screen
//! timeouts ([`run_sleep_check`]).
//!
//! The tray runs [`run_hibernate`] on a new thread for each `Command::Hibernate` and posts what it returns as
//! `Event::Done(Outcome::Hibernate(..))`. When the hibernate works, the call returns only after the PC resumed
//! (research, "Lid, sleep, hibernate and resume"), so the thread lives through the hibernate.

use std::fmt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::sync::Arc;
#[cfg(any(test, feature = "test-support"))]
use std::sync::Mutex;

use wakeshot_core::command::{CountdownReason, HibernateEnd, HibernateSkip, SoundEnd};
use wakeshot_core::policy::{ending, floor};
use wakeshot_core::view::Cue;
use windows::Win32::Foundation::{
    ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, LUID, SetLastError, WIN32_ERROR,
};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
    SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
};
use windows::Win32::System::Power::{IsPwrHibernateAllowed, SetSuspendState};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::PCWSTR;

use crate::log::{Level, Log};
use crate::power::system;
use crate::sound::SoundPlayer;
use crate::{WinError, clock};

const AREA: &str = "hibernate";

/// How long the lid check waits for the first callback of its fresh registration. Microsoft says it comes only once
/// Windows knows a lid device and its state, so an unknown lid must end the wait (architecture section 5).
pub const LID_WAIT_MS: u64 = 2_000;

/// What the last check reads and the calls after it, so the order and every fallback are tested without Windows
/// doing anything. `Send + Sync`, so the tray can keep one `Arc<dyn HibernateDeps>` and hand it to each hibernate
/// thread. `Win32Hibernate` is the real one, `DryRunHibernate` reads the real values and only logs the two calls that
/// change something, `FakeHibernate` (feature `test-support`) is scripted.
pub trait HibernateDeps: Send + Sync {
    /// `Some(true)` closed, `Some(false)` open, `None` unknown.
    fn lid_closed(&self) -> Option<bool>;
    /// Input idle as Windows reports it, suspended time included (research, "Clock").
    fn input_idle_ms(&self) -> Result<u64, WinError>;
    /// The clock `last_resume_mono_ms` comes from: the core's `Now::mono_ms`.
    fn mono_now_ms(&self) -> u64;
    /// On AC, and the battery percent when Windows knows it. An unknown source reads as AC.
    fn power_status(&self) -> Result<(bool, Option<u8>), WinError>;
    /// SeShutdownPrivilege enabled in this process's token, which `SetSuspendState` needs (Microsoft docs).
    fn enable_shutdown_privilege(&self) -> Result<(), WinError>;
    fn hibernate_allowed(&self) -> bool;
    /// Hibernates. `Ok` comes only after the PC resumed.
    fn hibernate(&self) -> Result<(), WinError>;
}

/// The hibernate thread's whole run for `Command::Hibernate { reason, floor_pct, last_resume_mono_ms }`.
///
/// In order, each step logged, the first that fails ends the run:
///
/// 1. the lid is closed: open is `Skipped(LidOpen)`, no answer within [`LID_WAIT_MS`] is `Skipped(LidUnknown)`;
/// 2. input idle, clamped at the last resume, is due by the core's own rule (`policy::ending::clamped_idle_ms` and
///    `check`, so 5 minutes): else `Skipped(InputSeen)`, also when the read fails;
/// 3. for `Floor` only: on battery (else or when the read fails `Skipped(OnAc)`) and below the floor by
///    `policy::floor::below` (else, an unknown percent too, `Skipped(AboveFloor)`);
/// 4. SeShutdownPrivilege: a failure is `Failed`;
/// 5. `IsPwrHibernateAllowed`: false is `NotAllowed`;
/// 6. the going to sleep sound, waited for; a failed sound is logged and the run goes on;
/// 7. the lid once more, as in step 1, so a lid opened while the sound played skips the hibernate;
/// 8. `SetSuspendState(TRUE, FALSE, FALSE)`: an error is `Failed`, a return after resume is `Resumed`.
///
/// An agent turn that starts during the sound is not seen: only the core knows of it, and nothing stops this thread.
/// Every failure is the safe side: no hibernate. The settings are already restored when the core sends `Hibernate`,
/// so the core's answer to a skip or a failure is a notice or another countdown, never a running laptop it holds.
pub fn run_hibernate(
    reason: CountdownReason,
    floor_pct: u8,
    last_resume_mono_ms: Option<u64>,
    deps: &dyn HibernateDeps,
    sound: &dyn SoundPlayer,
    log: &dyn Log,
) -> HibernateEnd {
    let info = |text: &str| log.line(Level::Info, AREA, text);
    let warn = |text: &str| log.line(Level::Warn, AREA, text);

    info(&format!(
        "last check before hibernate, {}",
        reason_text(reason)
    ));
    let checks = check_lid(deps, log, false)
        .and_then(|()| check_idle(last_resume_mono_ms, deps, log))
        .and_then(|()| match reason {
            CountdownReason::Floor => check_floor(floor_pct, deps, log),
            CountdownReason::WorkEnded => Ok(()),
        });
    if let Err(skip) = checks {
        return HibernateEnd::Skipped(skip);
    }

    if let Err(e) = deps.enable_shutdown_privilege() {
        warn(&format!("{}: no hibernate", failed(e)));
        return HibernateEnd::Failed(e.into());
    }
    info("SeShutdownPrivilege enabled");

    if !deps.hibernate_allowed() {
        warn("IsPwrHibernateAllowed is false (hibernate off or not supported): no hibernate");
        return HibernateEnd::NotAllowed;
    }
    info("hibernate allowed");

    // A sound is only a hint: a failed one must not keep a closed laptop running in a bag.
    match sound.play_and_wait(Cue::GoingToSleep) {
        Ok(SoundEnd::Played) => info("going to sleep sound played"),
        Ok(SoundEnd::Skipped) => {
            info("going to sleep sound skipped: the sound scheme is No sounds")
        }
        Err(e) => warn(&format!(
            "going to sleep sound: {}; hibernating anyway",
            failed(e)
        )),
    }

    // The sound plays for seconds: a user who hears it may open the lid to stop the sleep.
    if let Err(skip) = check_lid(deps, log, true) {
        return HibernateEnd::Skipped(skip);
    }

    // Logged before the call: if the PC never comes back, the log still shows how far the run got.
    info("calling SetSuspendState(TRUE, FALSE, FALSE)");
    match deps.hibernate() {
        Ok(()) => {
            info("SetSuspendState returned TRUE: the PC resumed");
            HibernateEnd::Resumed
        }
        Err(e) => {
            warn(&format!("{}: no hibernate", failed(e)));
            HibernateEnd::Failed(e.into())
        }
    }
}

/// The hibernate thread's run for `Command::SleepCheck`: steps 1 to 3, 6 and 7 of [`run_hibernate`], with no
/// privilege, no hibernate check and no call. So the going to sleep sound plays right before the core sets the
/// 1-minute timeouts, and a user who hears it can still open the lid. An unknown lid does not stop it: it is logged,
/// and the core's own lid reading decides (`Command::SleepCheck`). `Ok`: the core writes the short timeouts next. A
/// failed sound is logged and the check goes on, as before a hibernate.
pub fn run_sleep_check(
    reason: CountdownReason,
    floor_pct: u8,
    last_resume_mono_ms: Option<u64>,
    deps: &dyn HibernateDeps,
    sound: &dyn SoundPlayer,
    log: &dyn Log,
) -> Result<(), HibernateSkip> {
    log.line(
        Level::Info,
        AREA,
        &format!(
            "sleep check before the 1-minute sleep and screen timeouts, {}",
            reason_text(reason)
        ),
    );
    let lid = |after_sound| match check_lid(deps, log, after_sound) {
        Err(HibernateSkip::LidUnknown) => {
            log.line(
                Level::Info,
                AREA,
                "the core's own lid reading says closed: the sleep check goes on",
            );
            Ok(())
        }
        other => other,
    };
    lid(false)
        .and_then(|()| check_idle(last_resume_mono_ms, deps, log))
        .and_then(|()| match reason {
            CountdownReason::Floor => check_floor(floor_pct, deps, log),
            CountdownReason::WorkEnded => Ok(()),
        })?;
    play_going_to_sleep(sound, log);
    lid(true)?;
    log.line(Level::Info, AREA, "sleep check passed");
    Ok(())
}

/// A sound is only a hint: a failed one must not keep a closed laptop running in a bag.
fn play_going_to_sleep(sound: &dyn SoundPlayer, log: &dyn Log) {
    match sound.play_and_wait(Cue::GoingToSleep) {
        Ok(SoundEnd::Played) => log.line(Level::Info, AREA, "going to sleep sound played"),
        Ok(SoundEnd::Skipped) => log.line(
            Level::Info,
            AREA,
            "going to sleep sound skipped: the sound scheme is No sounds",
        ),
        Err(e) => log.line(
            Level::Warn,
            AREA,
            &format!("going to sleep sound: {}; going on anyway", failed(e)),
        ),
    }
}

/// `after_sound`: the second check, right before the call; its lines say so.
fn check_lid(
    deps: &dyn HibernateDeps,
    log: &dyn Log,
    after_sound: bool,
) -> Result<(), HibernateSkip> {
    let when = if after_sound { " after the sound" } else { "" };
    match deps.lid_closed() {
        Some(true) => {
            log.line(Level::Info, AREA, &format!("lid closed{when}"));
            Ok(())
        }
        Some(false) => {
            log.line(Level::Info, AREA, &format!("lid open{when}: no hibernate"));
            Err(HibernateSkip::LidOpen)
        }
        None => {
            log.line(
                Level::Warn,
                AREA,
                &format!("no lid state within {LID_WAIT_MS} ms{when}: no hibernate"),
            );
            Err(HibernateSkip::LidUnknown)
        }
    }
}

/// The core's own rule, so this check skips only for input that came after the core's reading (`policy::ending`).
fn check_idle(
    last_resume_mono_ms: Option<u64>,
    deps: &dyn HibernateDeps,
    log: &dyn Log,
) -> Result<(), HibernateSkip> {
    let reported = match deps.input_idle_ms() {
        Ok(ms) => ms,
        Err(e) => {
            // An idle time nobody could read is never 5 minutes without input.
            log.line(
                Level::Warn,
                AREA,
                &format!("{}: counted as input, no hibernate", failed(e)),
            );
            return Err(HibernateSkip::InputSeen);
        }
    };
    let now = deps.mono_now_ms();
    let idle = ending::clamped_idle_ms(reported, now, last_resume_mono_ms);
    let resume = match last_resume_mono_ms {
        Some(resumed) => format!(
            "{} s since the last resume",
            now.saturating_sub(resumed) / 1000
        ),
        None => "no resume since start".to_owned(),
    };
    let reading = format!(
        "input idle {} s (Windows reports {} s; {resume})",
        idle / 1000,
        reported / 1000
    );
    if ending::check(idle, now) == ending::Check::Due {
        log.line(
            Level::Info,
            AREA,
            &format!("{reading}: at least {} s", ending::IDLE_MS / 1000),
        );
        Ok(())
    } else {
        log.line(
            Level::Info,
            AREA,
            &format!(
                "{reading}: under {} s, no hibernate",
                ending::IDLE_MS / 1000
            ),
        );
        Err(HibernateSkip::InputSeen)
    }
}

/// The battery floor rule of the core (`policy::floor::below`), so the two never drift apart.
fn check_floor(
    floor_pct: u8,
    deps: &dyn HibernateDeps,
    log: &dyn Log,
) -> Result<(), HibernateSkip> {
    let (ac, battery_pct) = match deps.power_status() {
        Ok(status) => status,
        Err(e) => {
            // As `power_status` reads an unknown source: AC, so a read nobody trusts never hibernates.
            log.line(
                Level::Warn,
                AREA,
                &format!("{}: counted as AC, no hibernate", failed(e)),
            );
            return Err(HibernateSkip::OnAc);
        }
    };
    if ac {
        log.line(Level::Info, AREA, "on AC: no hibernate");
        return Err(HibernateSkip::OnAc);
    }
    let pct = battery_pct.map_or_else(|| "unknown".to_owned(), |p| format!("{p}%"));
    if floor::below(battery_pct, floor_pct) {
        log.line(
            Level::Info,
            AREA,
            &format!("on battery at {pct}, below the floor of {floor_pct}%"),
        );
        Ok(())
    } else {
        log.line(
            Level::Info,
            AREA,
            &format!("on battery at {pct}, not below the floor of {floor_pct}%: no hibernate"),
        );
        Err(HibernateSkip::AboveFloor)
    }
}

fn reason_text(reason: CountdownReason) -> &'static str {
    match reason {
        CountdownReason::WorkEnded => "work ended with the lid closed",
        CountdownReason::Floor => "battery under the floor with the lid closed",
    }
}

fn failed(e: WinError) -> String {
    format!("{} failed with {}", e.call, e.code)
}

/// The real calls.
#[derive(Clone, Copy, Debug, Default)]
pub struct Win32Hibernate;

impl HibernateDeps for Win32Hibernate {
    fn lid_closed(&self) -> Option<bool> {
        system::lid_once(LID_WAIT_MS)
    }

    fn input_idle_ms(&self) -> Result<u64, WinError> {
        clock::input_idle_ms()
    }

    fn mono_now_ms(&self) -> u64 {
        clock::now().mono_ms
    }

    fn power_status(&self) -> Result<(bool, Option<u8>), WinError> {
        system::power_status()
    }

    fn enable_shutdown_privilege(&self) -> Result<(), WinError> {
        enable_shutdown_privilege()
    }

    fn hibernate_allowed(&self) -> bool {
        // SAFETY: no arguments; it only returns a value.
        unsafe { IsPwrHibernateAllowed() }
    }

    fn hibernate(&self) -> Result<(), WinError> {
        // A slip in a test must never hibernate the laptop the tests run on.
        if cfg!(test) {
            panic!("a test called the real SetSuspendState");
        }
        set_suspend_state()
    }
}

/// Standard tokens hold SeShutdownPrivilege disabled; a process started by Explorer can enable it (research, "Lid,
/// sleep, hibernate and resume").
fn enable_shutdown_privilege() -> Result<(), WinError> {
    let mut raw = HANDLE::default();
    // SAFETY: the current-process pseudo handle needs no close, and `raw` is a live out pointer.
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &mut raw) }
        .map_err(|e| win_error("OpenProcessToken", &e))?;
    // SAFETY: a handle the call just opened, owned by nothing else; the drop closes it once.
    let token = unsafe { OwnedHandle::from_raw_handle(raw.0) };

    let mut luid = LUID::default();
    // SAFETY: a null system name means this PC; the name is a static string and `luid` a live out pointer.
    unsafe { LookupPrivilegeValueW(PCWSTR::null(), SE_SHUTDOWN_NAME, &mut luid) }
        .map_err(|e| win_error("LookupPrivilegeValueW", &e))?;

    let privileges = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    // SAFETY: takes a value and returns nothing.
    unsafe { SetLastError(WIN32_ERROR(0)) };
    // SAFETY: the token is open with TOKEN_ADJUST_PRIVILEGES, `privileges` outlives the call, and no previous state
    // is asked for, so no buffer is needed.
    unsafe {
        AdjustTokenPrivileges(
            HANDLE(token.as_raw_handle()),
            false,
            Some(&privileges),
            0,
            None,
            None,
        )
    }
    .map_err(|e| win_error("AdjustTokenPrivileges", &e))?;
    // Microsoft: a success with ERROR_NOT_ALL_ASSIGNED means the token does not hold the privilege.
    // SAFETY: takes no arguments and only returns a value.
    let code = unsafe { GetLastError() };
    if code == ERROR_NOT_ALL_ASSIGNED {
        return Err(WinError {
            call: "AdjustTokenPrivileges",
            code: code.0,
        });
    }
    Ok(())
}

/// `bHibernate` TRUE, though FALSE hibernates this Modern Standby laptop too (research): on a PC with S3, FALSE would
/// sleep instead. `bForce` has no effect (Microsoft docs). `bWakeupEventsDisabled` FALSE: which wakes TRUE turns off,
/// the power button among them or not, is not verified.
fn set_suspend_state() -> Result<(), WinError> {
    // SAFETY: takes a value and returns nothing.
    unsafe { SetLastError(WIN32_ERROR(0)) };
    // SAFETY: plain values in and out; the call blocks until the PC resumed.
    if unsafe { SetSuspendState(true, false, false) } {
        return Ok(());
    }
    Err(WinError {
        call: "SetSuspendState",
        // SAFETY: takes no arguments and only returns a value.
        code: unsafe { GetLastError() }.0,
    })
}

/// Win32 errors keep their Win32 code (5, 1300), so a log line matches the docs; other failures keep the HRESULT.
fn win_error(call: &'static str, error: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: WIN32_ERROR::from_error(error).map_or(error.code().0 as u32, |e| e.0),
    }
}

/// The dry run: the reads are `reads`' own, so the checks run on real values, and the two calls that change
/// something, the privilege and `SetSuspendState`, are only logged. It never hibernates: a run whose checks all pass
/// answers `Resumed` at once.
pub struct DryRunHibernate<D = Win32Hibernate> {
    reads: D,
    log: Arc<dyn Log>,
}

impl DryRunHibernate {
    /// Over the real reads.
    pub fn new(log: Arc<dyn Log>) -> Self {
        Self::over(Win32Hibernate, log)
    }
}

impl<D: HibernateDeps> DryRunHibernate<D> {
    /// Over any reads, so a test can pass every check and still see that nothing hibernates.
    pub fn over(reads: D, log: Arc<dyn Log>) -> Self {
        Self { reads, log }
    }

    fn line(&self, text: &str) {
        self.log
            .line(Level::Info, AREA, &format!("dry run: {text}"));
    }
}

impl<D: fmt::Debug> fmt::Debug for DryRunHibernate<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DryRunHibernate")
            .field("reads", &self.reads)
            .finish_non_exhaustive()
    }
}

impl<D: HibernateDeps> HibernateDeps for DryRunHibernate<D> {
    fn lid_closed(&self) -> Option<bool> {
        self.reads.lid_closed()
    }

    fn input_idle_ms(&self) -> Result<u64, WinError> {
        self.reads.input_idle_ms()
    }

    fn mono_now_ms(&self) -> u64 {
        self.reads.mono_now_ms()
    }

    fn power_status(&self) -> Result<(bool, Option<u8>), WinError> {
        self.reads.power_status()
    }

    fn enable_shutdown_privilege(&self) -> Result<(), WinError> {
        self.line("AdjustTokenPrivileges(SeShutdownPrivilege)");
        Ok(())
    }

    fn hibernate_allowed(&self) -> bool {
        self.reads.hibernate_allowed()
    }

    fn hibernate(&self) -> Result<(), WinError> {
        self.line("SetSuspendState(TRUE, FALSE, FALSE)");
        Ok(())
    }
}

/// One call a `FakeHibernate` saw.
#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HibernateCall {
    Lid,
    InputIdle,
    MonoNow,
    PowerStatus,
    Privilege,
    Allowed,
    Hibernate,
}

/// Answers each call with its field and records it in `calls`. The default passes every check and hibernates: lid
/// closed, 10 minutes idle, on battery at 10%, privilege and hibernate allowed, and a hibernate that returns `Ok`; a
/// test changes the one field it is about.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug)]
pub struct FakeHibernate {
    pub lid: Option<bool>,
    pub idle_ms: Result<u64, WinError>,
    pub mono_ms: u64,
    pub power: Result<(bool, Option<u8>), WinError>,
    pub privilege: Result<(), WinError>,
    pub allowed: bool,
    pub hibernate: Result<(), WinError>,
    pub calls: Mutex<Vec<HibernateCall>>,
}

#[cfg(any(test, feature = "test-support"))]
impl Default for FakeHibernate {
    fn default() -> Self {
        Self {
            lid: Some(true),
            idle_ms: Ok(10 * 60 * 1000),
            mono_ms: 7_200_000,
            power: Ok((false, Some(10))),
            privilege: Ok(()),
            allowed: true,
            hibernate: Ok(()),
            calls: Mutex::new(Vec::new()),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl FakeHibernate {
    pub fn calls(&self) -> Vec<HibernateCall> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn saw(&self, call: HibernateCall) {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(call);
    }
}

#[cfg(any(test, feature = "test-support"))]
impl HibernateDeps for FakeHibernate {
    fn lid_closed(&self) -> Option<bool> {
        self.saw(HibernateCall::Lid);
        self.lid
    }

    fn input_idle_ms(&self) -> Result<u64, WinError> {
        self.saw(HibernateCall::InputIdle);
        self.idle_ms
    }

    fn mono_now_ms(&self) -> u64 {
        self.saw(HibernateCall::MonoNow);
        self.mono_ms
    }

    fn power_status(&self) -> Result<(bool, Option<u8>), WinError> {
        self.saw(HibernateCall::PowerStatus);
        self.power
    }

    fn enable_shutdown_privilege(&self) -> Result<(), WinError> {
        self.saw(HibernateCall::Privilege);
        self.privilege
    }

    fn hibernate_allowed(&self) -> bool {
        self.saw(HibernateCall::Allowed);
        self.allowed
    }

    fn hibernate(&self) -> Result<(), WinError> {
        self.saw(HibernateCall::Hibernate);
        self.hibernate
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use wakeshot_core::command::Failure;
    use windows::Win32::Security::{
        GetTokenInformation, TOKEN_ACCESS_MASK, TOKEN_PRIVILEGES_ATTRIBUTES, TOKEN_QUERY,
        TokenPrivileges,
    };
    use windows::Win32::System::Power::{GetPwrCapabilities, SYSTEM_POWER_CAPABILITIES};

    use super::*;
    use crate::log::MemoryLog;
    use crate::sound::FakeSoundPlayer;
    use CountdownReason::{Floor, WorkEnded};
    use HibernateCall::{Allowed, Hibernate, InputIdle, Lid, MonoNow, PowerStatus, Privilege};

    const FLOOR: u8 = 20;
    const MINUTE: u64 = 60_000;
    /// `FakeHibernate`'s mono now.
    const T0: u64 = 7_200_000;

    /// Every call of a run whose checks all pass, for the `Floor` reason. The second `Lid` comes after the sound.
    const ALL_FLOOR: [HibernateCall; 8] = [
        Lid,
        InputIdle,
        MonoNow,
        PowerStatus,
        Privilege,
        Allowed,
        Lid,
        Hibernate,
    ];

    fn error(call: &'static str, code: u32) -> WinError {
        WinError { call, code }
    }

    struct Run {
        end: HibernateEnd,
        played: Vec<Cue>,
        lines: Vec<(Level, &'static str, String)>,
    }

    fn run_with(
        reason: CountdownReason,
        last_resume: Option<u64>,
        fake: &FakeHibernate,
        sound: &FakeSoundPlayer,
    ) -> Run {
        let log = MemoryLog::default();
        let end = run_hibernate(reason, FLOOR, last_resume, fake, sound, &log);
        Run {
            end,
            played: sound.played.lock().unwrap().clone(),
            lines: log.lines.lock().unwrap().clone(),
        }
    }

    fn run(reason: CountdownReason, fake: &FakeHibernate) -> Run {
        run_with(reason, None, fake, &FakeSoundPlayer::default())
    }

    #[test]
    fn lid_open_skips() {
        let fake = FakeHibernate {
            lid: Some(false),
            ..FakeHibernate::default()
        };
        let r = run(Floor, &fake);
        assert_eq!(r.end, HibernateEnd::Skipped(HibernateSkip::LidOpen));
        assert_eq!(fake.calls(), [Lid]);
    }

    #[test]
    fn lid_unknown_skips() {
        let fake = FakeHibernate {
            lid: None,
            ..FakeHibernate::default()
        };
        let r = run(WorkEnded, &fake);
        assert_eq!(r.end, HibernateEnd::Skipped(HibernateSkip::LidUnknown));
        assert_eq!(fake.calls(), [Lid]);
        assert!(
            r.lines
                .iter()
                .any(|(level, _, text)| *level == Level::Warn && text.contains("no lid state")),
            "{:?}",
            r.lines
        );
    }

    #[test]
    fn input_seen_skips() {
        let fake = FakeHibernate {
            idle_ms: Ok(ending::IDLE_MS - 1),
            ..FakeHibernate::default()
        };
        let r = run(WorkEnded, &fake);
        assert_eq!(r.end, HibernateEnd::Skipped(HibernateSkip::InputSeen));
        assert_eq!(fake.calls(), [Lid, InputIdle, MonoNow]);

        // Exactly 5 minutes is due, as for the core's countdown.
        let fake = FakeHibernate {
            idle_ms: Ok(ending::IDLE_MS),
            ..FakeHibernate::default()
        };
        assert_eq!(run(WorkEnded, &fake).end, HibernateEnd::Resumed);
    }

    #[test]
    fn recent_resume_counts_as_input() {
        // Windows reports 20 minutes, but the PC resumed a minute ago.
        let fake = FakeHibernate {
            idle_ms: Ok(20 * MINUTE),
            ..FakeHibernate::default()
        };
        let sound = FakeSoundPlayer::default();
        let r = run_with(WorkEnded, Some(T0 - MINUTE), &fake, &sound);
        assert_eq!(r.end, HibernateEnd::Skipped(HibernateSkip::InputSeen));

        // Five minutes after the resume it is due.
        let fake = FakeHibernate {
            idle_ms: Ok(20 * MINUTE),
            ..FakeHibernate::default()
        };
        let r = run_with(WorkEnded, Some(T0 - ending::IDLE_MS), &fake, &sound);
        assert_eq!(r.end, HibernateEnd::Resumed);

        // A resume stamped after now (a clock read out of order) is no idle at all.
        let fake = FakeHibernate {
            idle_ms: Ok(20 * MINUTE),
            ..FakeHibernate::default()
        };
        let r = run_with(WorkEnded, Some(T0 + 1), &fake, &sound);
        assert_eq!(r.end, HibernateEnd::Skipped(HibernateSkip::InputSeen));
    }

    #[test]
    fn idle_read_failure_counts_as_input() {
        let fake = FakeHibernate {
            idle_ms: Err(error("GetLastInputInfo", 0)),
            ..FakeHibernate::default()
        };
        let r = run(WorkEnded, &fake);
        assert_eq!(r.end, HibernateEnd::Skipped(HibernateSkip::InputSeen));
        assert_eq!(fake.calls(), [Lid, InputIdle]);
    }

    #[test]
    fn floor_on_ac_skips() {
        let fake = FakeHibernate {
            power: Ok((true, Some(5))),
            ..FakeHibernate::default()
        };
        let r = run(Floor, &fake);
        assert_eq!(r.end, HibernateEnd::Skipped(HibernateSkip::OnAc));
        assert_eq!(fake.calls(), [Lid, InputIdle, MonoNow, PowerStatus]);

        // The read failing counts as AC.
        let fake = FakeHibernate {
            power: Err(error("GetSystemPowerStatus", 31)),
            ..FakeHibernate::default()
        };
        assert_eq!(
            run(Floor, &fake).end,
            HibernateEnd::Skipped(HibernateSkip::OnAc)
        );
    }

    #[test]
    fn floor_above_skips() {
        let with_pct = |pct| FakeHibernate {
            power: Ok((false, pct)),
            ..FakeHibernate::default()
        };
        // At the floor is not below it.
        for pct in [Some(FLOOR), Some(FLOOR + 1), Some(100), None] {
            let fake = with_pct(pct);
            assert_eq!(
                run(Floor, &fake).end,
                HibernateEnd::Skipped(HibernateSkip::AboveFloor),
                "{pct:?}"
            );
            assert_eq!(fake.calls(), [Lid, InputIdle, MonoNow, PowerStatus]);
        }
        assert_eq!(
            run(Floor, &with_pct(Some(FLOOR - 1))).end,
            HibernateEnd::Resumed
        );
    }

    #[test]
    fn work_ended_never_reads_the_power_source() {
        let fake = FakeHibernate {
            power: Ok((true, Some(100))),
            ..FakeHibernate::default()
        };
        let r = run(WorkEnded, &fake);
        assert_eq!(r.end, HibernateEnd::Resumed);
        assert!(!fake.calls().contains(&PowerStatus), "{:?}", fake.calls());
    }

    /// Each way a run stops, with the calls it made up to and including the one that stopped it.
    fn stops() -> Vec<(&'static str, FakeHibernate, HibernateEnd, usize)> {
        let base = FakeHibernate::default;
        vec![
            (
                "lid",
                FakeHibernate {
                    lid: Some(false),
                    ..base()
                },
                HibernateEnd::Skipped(HibernateSkip::LidOpen),
                1,
            ),
            (
                "idle",
                FakeHibernate {
                    idle_ms: Ok(MINUTE),
                    ..base()
                },
                HibernateEnd::Skipped(HibernateSkip::InputSeen),
                3,
            ),
            (
                "power",
                FakeHibernate {
                    power: Ok((false, Some(FLOOR))),
                    ..base()
                },
                HibernateEnd::Skipped(HibernateSkip::AboveFloor),
                4,
            ),
            (
                "privilege",
                FakeHibernate {
                    privilege: Err(error("AdjustTokenPrivileges", 1300)),
                    ..base()
                },
                HibernateEnd::Failed(Failure {
                    call: "AdjustTokenPrivileges",
                    code: 1300,
                }),
                5,
            ),
            (
                "allowed",
                FakeHibernate {
                    allowed: false,
                    ..base()
                },
                HibernateEnd::NotAllowed,
                6,
            ),
            (
                "hibernate",
                FakeHibernate {
                    hibernate: Err(error("SetSuspendState", 5)),
                    ..base()
                },
                HibernateEnd::Failed(Failure {
                    call: "SetSuspendState",
                    code: 5,
                }),
                8,
            ),
        ]
    }

    #[test]
    fn checks_run_in_order_and_stop_at_first_fail() {
        let fake = FakeHibernate::default();
        assert_eq!(run(Floor, &fake).end, HibernateEnd::Resumed);
        assert_eq!(fake.calls(), ALL_FLOOR);

        for (name, fake, end, made) in stops() {
            assert_eq!(run(Floor, &fake).end, end, "{name}");
            assert_eq!(fake.calls(), ALL_FLOOR[..made], "{name}");
        }
    }

    /// Records the fake's calls at the moment the sound plays.
    struct OrderSound<'a> {
        fake: &'a FakeHibernate,
        heard_after: Mutex<Vec<Vec<HibernateCall>>>,
    }

    impl SoundPlayer for OrderSound<'_> {
        fn play(&self, cue: Cue) {
            panic!("the hibernate thread only plays and waits, got play({cue:?})");
        }

        fn play_and_wait(&self, cue: Cue) -> Result<SoundEnd, WinError> {
            assert_eq!(cue, Cue::GoingToSleep);
            self.heard_after.lock().unwrap().push(self.fake.calls());
            Ok(SoundEnd::Played)
        }
    }

    #[test]
    fn sound_plays_only_when_all_checks_pass() {
        let fake = FakeHibernate::default();
        let sound = OrderSound {
            fake: &fake,
            heard_after: Mutex::default(),
        };
        let log = MemoryLog::default();
        assert_eq!(
            run_hibernate(Floor, FLOOR, None, &fake, &sound, &log),
            HibernateEnd::Resumed
        );
        assert_eq!(
            *sound.heard_after.lock().unwrap(),
            [ALL_FLOOR[..6].to_vec()],
            "once, after every check and right before the call"
        );

        for (name, fake, end, _) in stops() {
            let r = run(Floor, &fake);
            assert_eq!(r.end, end, "{name}");
            let expected: &[Cue] = if name == "hibernate" {
                &[Cue::GoingToSleep]
            } else {
                &[]
            };
            assert_eq!(r.played, expected, "{name}");
        }
    }

    /// The fake's answers, but the lid reads `after` once the sound has played: the user opened it while the sound
    /// played, or its state got lost.
    struct LidDuringSound {
        fake: FakeHibernate,
        after: Option<bool>,
        played: AtomicBool,
    }

    impl HibernateDeps for LidDuringSound {
        fn lid_closed(&self) -> Option<bool> {
            let before = self.fake.lid_closed();
            if self.played.load(Ordering::SeqCst) {
                self.after
            } else {
                before
            }
        }

        fn input_idle_ms(&self) -> Result<u64, WinError> {
            self.fake.input_idle_ms()
        }

        fn mono_now_ms(&self) -> u64 {
            self.fake.mono_now_ms()
        }

        fn power_status(&self) -> Result<(bool, Option<u8>), WinError> {
            self.fake.power_status()
        }

        fn enable_shutdown_privilege(&self) -> Result<(), WinError> {
            self.fake.enable_shutdown_privilege()
        }

        fn hibernate_allowed(&self) -> bool {
            self.fake.hibernate_allowed()
        }

        fn hibernate(&self) -> Result<(), WinError> {
            self.fake.hibernate()
        }
    }

    impl SoundPlayer for LidDuringSound {
        fn play(&self, cue: Cue) {
            panic!("the hibernate thread only plays and waits, got play({cue:?})");
        }

        fn play_and_wait(&self, cue: Cue) -> Result<SoundEnd, WinError> {
            assert_eq!(cue, Cue::GoingToSleep);
            self.played.store(true, Ordering::SeqCst);
            Ok(SoundEnd::Played)
        }
    }

    /// The lid is read once more after the sound, right before the call.
    #[test]
    fn lid_opened_during_the_sound_skips() {
        for (after, skip, level, line) in [
            (
                Some(false),
                HibernateSkip::LidOpen,
                Level::Info,
                "lid open after the sound: no hibernate",
            ),
            (
                None,
                HibernateSkip::LidUnknown,
                Level::Warn,
                "no lid state within 2000 ms after the sound: no hibernate",
            ),
        ] {
            let deps = LidDuringSound {
                fake: FakeHibernate::default(),
                after,
                played: AtomicBool::new(false),
            };
            let log = MemoryLog::default();
            let end = run_hibernate(Floor, FLOOR, None, &deps, &deps, &log);
            assert_eq!(end, HibernateEnd::Skipped(skip), "{after:?}");
            assert!(deps.played.load(Ordering::SeqCst), "{after:?}");
            assert_eq!(
                deps.fake.calls(),
                ALL_FLOOR[..7],
                "{after:?}: no hibernate call"
            );
            let lines = log.lines.lock().unwrap();
            assert_eq!(
                lines.last().map(|(l, _, text)| (*l, text.as_str())),
                Some((level, line)),
                "{after:?}"
            );
        }
    }

    #[test]
    fn sound_failure_still_hibernates() {
        let fake = FakeHibernate::default();
        let sound = FakeSoundPlayer {
            wait_error: Some(error("IAudioClient::Start", 0x8889_0004)),
            ..FakeSoundPlayer::default()
        };
        let r = run_with(WorkEnded, None, &fake, &sound);
        assert_eq!(r.end, HibernateEnd::Resumed);
        assert_eq!(r.played, [Cue::GoingToSleep]);
        assert!(fake.calls().contains(&Hibernate));
        assert!(
            r.lines
                .iter()
                .any(|(level, _, text)| *level == Level::Warn
                    && text.contains("IAudioClient::Start")),
            "{:?}",
            r.lines
        );
    }

    #[test]
    fn privilege_failure_fails() {
        let fake = FakeHibernate {
            privilege: Err(error("AdjustTokenPrivileges", 1300)),
            ..FakeHibernate::default()
        };
        let r = run(WorkEnded, &fake);
        assert_eq!(
            r.end,
            HibernateEnd::Failed(Failure {
                call: "AdjustTokenPrivileges",
                code: 1300
            })
        );
        assert_eq!(r.played, []);
        assert!(!fake.calls().contains(&Hibernate));
    }

    #[test]
    fn not_allowed() {
        let fake = FakeHibernate {
            allowed: false,
            ..FakeHibernate::default()
        };
        let r = run(WorkEnded, &fake);
        assert_eq!(r.end, HibernateEnd::NotAllowed);
        assert_eq!(
            r.played,
            [],
            "no going to sleep sound for a sleep that cannot come"
        );
        assert_eq!(fake.calls(), [Lid, InputIdle, MonoNow, Privilege, Allowed]);
    }

    #[test]
    fn call_failed() {
        let fake = FakeHibernate {
            hibernate: Err(error("SetSuspendState", 1314)),
            ..FakeHibernate::default()
        };
        let r = run(Floor, &fake);
        assert_eq!(
            r.end,
            HibernateEnd::Failed(Failure {
                call: "SetSuspendState",
                code: 1314
            })
        );
        assert_eq!(fake.calls(), ALL_FLOOR);
        assert_eq!(
            r.lines
                .last()
                .map(|(level, _, text)| (*level, text.as_str())),
            Some((
                Level::Warn,
                "SetSuspendState failed with 1314: no hibernate"
            ))
        );
    }

    #[test]
    fn resumed() {
        let fake = FakeHibernate::default();
        let r = run(WorkEnded, &fake);
        assert_eq!(r.end, HibernateEnd::Resumed);
        assert_eq!(r.played, [Cue::GoingToSleep]);
        assert_eq!(
            fake.calls(),
            [Lid, InputIdle, MonoNow, Privilege, Allowed, Lid, Hibernate]
        );
    }

    #[test]
    fn each_step_is_logged() {
        let fake = FakeHibernate::default();
        let sound = FakeSoundPlayer::default();
        let r = run_with(Floor, Some(T0 - 30 * MINUTE), &fake, &sound);
        let text: Vec<&str> = r.lines.iter().map(|(_, _, t)| t.as_str()).collect();
        assert_eq!(
            text,
            [
                "last check before hibernate, battery under the floor with the lid closed",
                "lid closed",
                "input idle 600 s (Windows reports 600 s; 1800 s since the last resume): at least 300 s",
                "on battery at 10%, below the floor of 20%",
                "SeShutdownPrivilege enabled",
                "hibernate allowed",
                "going to sleep sound played",
                "lid closed after the sound",
                "calling SetSuspendState(TRUE, FALSE, FALSE)",
                "SetSuspendState returned TRUE: the PC resumed",
            ]
        );
        assert!(
            r.lines
                .iter()
                .all(|(level, area, _)| (*level, *area) == (Level::Info, AREA))
        );
    }

    #[test]
    fn dry_run_never_hibernates() {
        let log = Arc::new(MemoryLog::default());
        let dry = DryRunHibernate::over(FakeHibernate::default(), log.clone());
        let end = run_hibernate(
            Floor,
            FLOOR,
            None,
            &dry,
            &FakeSoundPlayer::default(),
            log.as_ref(),
        );
        assert_eq!(end, HibernateEnd::Resumed, "every check passed");
        assert_eq!(
            dry.reads.calls(),
            [Lid, InputIdle, MonoNow, PowerStatus, Allowed, Lid],
            "only the reads reach the inner deps"
        );
        let lines = log.lines.lock().unwrap();
        let dry_lines: Vec<&str> = lines
            .iter()
            .map(|(_, _, t)| t.as_str())
            .filter(|t| t.starts_with("dry run: "))
            .collect();
        assert_eq!(
            dry_lines,
            [
                "dry run: AdjustTokenPrivileges(SeShutdownPrivilege)",
                "dry run: SetSuspendState(TRUE, FALSE, FALSE)",
            ]
        );
        drop(lines);

        // Its checks still skip on real-looking values.
        let dry = DryRunHibernate::over(
            FakeHibernate {
                lid: Some(false),
                ..FakeHibernate::default()
            },
            log.clone(),
        );
        assert_eq!(
            run_hibernate(
                WorkEnded,
                FLOOR,
                None,
                &dry,
                &FakeSoundPlayer::default(),
                log.as_ref()
            ),
            HibernateEnd::Skipped(HibernateSkip::LidOpen)
        );
    }

    #[test]
    fn deps_go_to_another_thread() {
        let deps: Arc<dyn HibernateDeps> = Arc::new(DryRunHibernate::over(
            FakeHibernate::default(),
            Arc::new(MemoryLog::default()),
        ));
        let sound: Arc<dyn SoundPlayer> = Arc::new(FakeSoundPlayer::default());
        let log: Arc<dyn Log> = Arc::new(MemoryLog::default());
        let end = std::thread::spawn(move || {
            run_hibernate(WorkEnded, FLOOR, None, &*deps, &*sound, &*log)
        })
        .join()
        .unwrap();
        assert_eq!(end, HibernateEnd::Resumed);
    }

    #[test]
    fn win32_errors_keep_their_code() {
        let e = windows::core::Error::from(WIN32_ERROR(1314));
        assert_eq!(
            win_error("SetSuspendState", &e),
            error("SetSuspendState", 1314)
        );
    }

    /// This process's token, and SeShutdownPrivilege's id.
    fn token_and_luid(access: TOKEN_ACCESS_MASK) -> (OwnedHandle, LUID) {
        let mut raw = HANDLE::default();
        // SAFETY: as in `enable_shutdown_privilege`.
        unsafe { OpenProcessToken(GetCurrentProcess(), access, &mut raw) }.unwrap();
        // SAFETY: a fresh handle owned by nothing else.
        let token = unsafe { OwnedHandle::from_raw_handle(raw.0) };
        let mut luid = LUID::default();
        // SAFETY: as in `enable_shutdown_privilege`.
        unsafe { LookupPrivilegeValueW(PCWSTR::null(), SE_SHUTDOWN_NAME, &mut luid) }.unwrap();
        (token, luid)
    }

    /// Turns the privilege off again, so the test sees the enable do the work and not an earlier state.
    fn disable_shutdown_privilege() {
        let (token, luid) = token_and_luid(TOKEN_ADJUST_PRIVILEGES);
        let off = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: TOKEN_PRIVILEGES_ATTRIBUTES(0),
            }],
        };
        // SAFETY: as in `enable_shutdown_privilege`.
        unsafe {
            AdjustTokenPrivileges(
                HANDLE(token.as_raw_handle()),
                false,
                Some(&off),
                0,
                None,
                None,
            )
        }
        .unwrap();
    }

    /// Whether this process's token has SeShutdownPrivilege enabled, read back with `GetTokenInformation`.
    fn shutdown_privilege_enabled() -> bool {
        let (token, luid) = token_and_luid(TOKEN_QUERY);

        // u64 words, so the buffer is aligned for TOKEN_PRIVILEGES.
        let mut buffer = vec![0u64; 512];
        let mut len = 0u32;
        // SAFETY: the buffer is live and its byte length is passed; `len` is a live out pointer.
        unsafe {
            GetTokenInformation(
                HANDLE(token.as_raw_handle()),
                TokenPrivileges,
                Some(buffer.as_mut_ptr().cast()),
                (buffer.len() * 8) as u32,
                &mut len,
            )
        }
        .unwrap();
        let privileges = buffer.as_ptr().cast::<TOKEN_PRIVILEGES>();
        // SAFETY: Windows wrote a TOKEN_PRIVILEGES with `PrivilegeCount` entries into the aligned buffer.
        unsafe {
            let count = (*privileges).PrivilegeCount as usize;
            let first = (&raw const (*privileges).Privileges).cast::<LUID_AND_ATTRIBUTES>();
            (0..count).map(|i| *first.add(i)).any(|p| {
                p.Luid.LowPart == luid.LowPart
                    && p.Luid.HighPart == luid.HighPart
                    && p.Attributes.0 & SE_PRIVILEGE_ENABLED.0 != 0
            })
        }
    }

    /// Real, harmless: turns a privilege the token already holds off and on, in this test process only, and reads it
    /// back. No other test here touches the token.
    #[test]
    fn privilege_enables() {
        disable_shutdown_privilege();
        assert!(!shutdown_privilege_enabled(), "off before the enable");
        assert_eq!(Win32Hibernate.enable_shutdown_privilege(), Ok(()));
        assert!(shutdown_privilege_enabled());
        assert_eq!(
            Win32Hibernate.enable_shutdown_privilege(),
            Ok(()),
            "a second enable"
        );
    }

    /// Real, read only. Microsoft: TRUE needs S4 support and Hiberfil.sys, and the value is also in `SystemS4`.
    #[test]
    fn hibernate_allowed_reads() {
        let allowed = Win32Hibernate.hibernate_allowed();
        let mut caps = SYSTEM_POWER_CAPABILITIES::default();
        // SAFETY: `caps` is a live local the call fills.
        assert!(unsafe { GetPwrCapabilities(&mut caps) });
        println!(
            "IsPwrHibernateAllowed {allowed}, SystemS4 {}, HiberFilePresent {}",
            caps.SystemS4, caps.HiberFilePresent
        );
        assert!(!allowed || caps.SystemS4, "allowed without S4");
        assert_eq!(Win32Hibernate.hibernate_allowed(), allowed, "a second read");
    }

    // The sleep check.

    fn check_with(
        reason: CountdownReason,
        fake: &FakeHibernate,
    ) -> (Result<(), HibernateSkip>, Vec<Cue>) {
        let sound = FakeSoundPlayer::default();
        let log = MemoryLog::default();
        let result = run_sleep_check(reason, FLOOR, None, fake, &sound, &log);
        let played = sound.played.lock().unwrap().clone();
        (result, played)
    }

    #[test]
    fn the_sleep_check_runs_the_checks_and_the_sound_and_never_hibernates() {
        let fake = FakeHibernate::default();
        let (result, played) = check_with(Floor, &fake);
        assert_eq!(result, Ok(()));
        assert_eq!(played, [Cue::GoingToSleep]);
        assert_eq!(fake.calls(), [Lid, InputIdle, MonoNow, PowerStatus, Lid]);

        let fake = FakeHibernate::default();
        let (result, _) = check_with(WorkEnded, &fake);
        assert_eq!(result, Ok(()));
        assert_eq!(fake.calls(), [Lid, InputIdle, MonoNow, Lid]);
    }

    #[test]
    fn the_sleep_check_skips_as_the_hibernate_check_does_before_the_sound() {
        let open = FakeHibernate {
            lid: Some(false),
            ..FakeHibernate::default()
        };
        let typed = FakeHibernate {
            idle_ms: Ok(ending::IDLE_MS - 1),
            ..FakeHibernate::default()
        };
        let on_ac = FakeHibernate {
            power: Ok((true, Some(5))),
            ..FakeHibernate::default()
        };
        let above = FakeHibernate {
            power: Ok((false, Some(FLOOR))),
            ..FakeHibernate::default()
        };
        for (fake, skip) in [
            (open, HibernateSkip::LidOpen),
            (typed, HibernateSkip::InputSeen),
            (on_ac, HibernateSkip::OnAc),
            (above, HibernateSkip::AboveFloor),
        ] {
            let (result, played) = check_with(Floor, &fake);
            assert_eq!(result, Err(skip));
            assert!(played.is_empty(), "{skip:?}: no sound before a skip");
        }
    }

    #[test]
    fn the_sleep_check_goes_on_with_the_lid_unknown_and_stops_for_a_lid_opened_during_the_sound() {
        let unknown = FakeHibernate {
            lid: None,
            ..FakeHibernate::default()
        };
        let (result, played) = check_with(WorkEnded, &unknown);
        assert_eq!(result, Ok(()));
        assert_eq!(played, [Cue::GoingToSleep]);

        let fake = LidDuringSound {
            fake: FakeHibernate::default(),
            after: Some(false),
            played: AtomicBool::new(false),
        };
        let sound_log = MemoryLog::default();
        let result = run_sleep_check(WorkEnded, FLOOR, None, &fake, &fake, &sound_log);
        assert_eq!(result, Err(HibernateSkip::LidOpen));
    }

    #[test]
    fn a_failed_sound_does_not_stop_the_sleep_check() {
        let fake = FakeHibernate::default();
        let sound = FakeSoundPlayer {
            wait_error: Some(error("waveOutOpen", 2)),
            ..FakeSoundPlayer::default()
        };
        let log = MemoryLog::default();
        let result = run_sleep_check(WorkEnded, FLOOR, None, &fake, &sound, &log);
        assert_eq!(result, Ok(()));
        let lines = log.lines.lock().unwrap().clone();
        assert!(
            lines
                .iter()
                .any(|(level, _, text)| *level == Level::Warn && text.contains("going on anyway")),
            "{lines:?}"
        );
    }
}
