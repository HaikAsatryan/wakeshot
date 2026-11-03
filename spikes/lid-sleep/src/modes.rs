use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use windows::Win32::System::Shutdown::LockWorkStation;

use crate::events::{self, Ev, Subscriptions};
use crate::plan::{self, PlanCopy};
use crate::power::{self, BOTH, Hold, WakeTimer};
use crate::ticks::{GapInfo, Ticker};
use crate::{Args, display, heat, log, sound, sys};

fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Local time `yyyy-MM-dd HH:mm:ss` a minute before now, for the System log query at the end.
fn events_since_now() -> String {
    let t = log::wall_clock();
    // Keep it simple: the query starts at the top of the current minute.
    let (date_time, _) = t.split_once('.').unwrap_or((&t, ""));
    format!("{}:00", &date_time[..date_time.len() - 3])
}

pub fn read() -> i32 {
    match power::capabilities() {
        Some(c) => {
            crate::log!(
                "caps",
                "LidPresent {}, S3 {}, S4 {}, HiberFilePresent {}, AoAc {}, AoAcConnectivity {}, FastSystemS4 {}, Hiberboot {}, WakeAlarmPresent {}, RtcWake S{}, AcOnLineWake S{}, SoftLidWake S{}, ThermalControl {}",
                c.LidPresent,
                c.SystemS3,
                c.SystemS4,
                c.HiberFilePresent,
                c.AoAc,
                c.AoAcConnectivitySupported,
                c.FastSystemS4,
                c.Hiberboot,
                c.WakeAlarmPresent,
                c.RtcWake.0 - 1,
                c.AcOnLineWake.0 - 1,
                c.SoftLidWake.0 - 1,
                c.ThermalControl
            );
        }
        None => crate::log!("caps", "GetPwrCapabilities failed"),
    }
    crate::log!(
        "caps",
        "IsPwrHibernateAllowed {}, IsPwrSuspendAllowed {}",
        power::hibernate_allowed(),
        power::suspend_allowed()
    );
    for v in [
        "HibernateEnabled",
        "HibernateEnabledDefault",
        "PlatformAoAcOverride",
        "CsEnabled",
    ] {
        crate::log!(
            "caps",
            "HKLM\\{}\\{v} = {:?}",
            sys::POWER_KEY,
            sys::reg_dword(sys::hklm(), sys::POWER_KEY, v)
        );
    }
    let (rc, out) = sys::run("powercfg", &["/a"]);
    crate::log!("caps", "powercfg /a rc {rc}");
    for l in out.lines().filter(|l| !l.trim().is_empty()) {
        crate::log!("caps", "  {}", l.trim());
    }
    plan::log_values("plan");

    let subs = Subscriptions::start(&events::DEFAULT_SETTINGS, false);
    events::nap(800);
    let first = events::drain();
    crate::log!("notify", "{} first callbacks within 800 ms", first.len());
    drop(subs);

    match display::active_targets() {
        Ok(ts) => {
            for t in &ts {
                crate::log!(
                    "display",
                    "active target: {} ({}), kind {:?}, name {}",
                    display::tech_name(t.tech),
                    t.tech,
                    display::kind(t.tech),
                    t.name
                );
            }
            crate::log!("display", "count: {}", display::count(&ts).short());
        }
        Err(rc) => crate::log!("display", "QueryDisplayConfig rc {rc}"),
    }
    let (_, out) = sys::powershell(
        r#"try { Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorConnectionParams -ErrorAction Stop | ForEach-Object { '{0} active={1} tech={2}' -f $_.InstanceName,$_.Active,$_.VideoOutputTechnology } } catch { 'WmiMonitorConnectionParams error: ' + $_.Exception.Message }"#,
    );
    for l in out.lines() {
        crate::log!("display", "WMI {l}");
    }

    match heat::Thermal::open().and_then(|t| t.sample()) {
        Ok(rs) => {
            for r in &rs {
                crate::log!(
                    "heat",
                    "PDH {} {} = {} (status {:#x})",
                    r.zone,
                    r.counter,
                    r.value,
                    r.status
                );
            }
            crate::log!("heat", "summary: {}", heat::summary(&rs));
        }
        Err(e) => crate::log!("heat", "PDH: {e}"),
    }
    let (_, out) = sys::powershell(
        r#"try { Get-CimInstance -ClassName Win32_PerfFormattedData_Counters_ThermalZoneInformation -ErrorAction Stop | ForEach-Object { '{0} T={1}K HP={2} passive={3}% throttle={4}' -f $_.Name,$_.Temperature,$_.HighPrecisionTemperature,$_.PercentPassiveLimit,$_.ThrottleReasons } } catch { 'ThermalZoneInformation error: ' + $_.Exception.Message }
try { Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature -ErrorAction Stop | ForEach-Object { '{0} current={1} critical={2} (tenths of K)' -f $_.InstanceName,$_.CurrentTemperature,$_.CriticalTripPoint } } catch { 'MSAcpi_ThermalZoneTemperature error: ' + $_.Exception.Message }"#,
    );
    for l in out.lines() {
        crate::log!("heat", "WMI {l}");
    }

    let (sleep, wake) = sys::last_sleep_wake();
    let now_i = sys::interrupt_ms();
    let now_u = sys::unbiased_ms();
    for (name, v) in [("LastSleepTime", sleep), ("LastWakeTime", wake)] {
        match v {
            Ok(t) => crate::log!(
                "ntpower",
                "{name} {t} (100 ns): {} s before now by interrupt time, {} s by unbiased time",
                (now_i as i64 - (t / 10_000) as i64) / 1000,
                (now_u as i64 - (t / 10_000) as i64) / 1000
            ),
            Err(st) => crate::log!("ntpower", "{name} status {st:#x}"),
        }
    }
    crate::log!(
        "ntpower",
        "interrupt time {} s, unbiased {} s (difference = time asleep since boot)",
        now_i / 1000,
        now_u / 1000
    );
    match sys::system_execution_state() {
        Ok(v) => crate::log!(
            "ntpower",
            "SystemExecutionState {v:#x} (ES_SYSTEM_REQUIRED 0x1, ES_DISPLAY_REQUIRED 0x2, ES_USER_PRESENT 0x4, ES_AWAYMODE 0x40)"
        ),
        Err(st) => crate::log!("ntpower", "SystemExecutionState status {st:#x}"),
    }
    match sys::system_power_information() {
        Ok(i) => crate::log!(
            "ntpower",
            "SystemPowerInformation MaxIdlenessAllowed {}, Idleness {}, TimeRemaining {} s, CoolingMode {}",
            i.MaxIdlenessAllowed,
            i.Idleness,
            i.TimeRemaining,
            i.CoolingMode.0
        ),
        Err(st) => crate::log!("ntpower", "SystemPowerInformation status {st:#x}"),
    }
    let (ok, e) = sys::enable_shutdown_privilege();
    crate::log!(
        "priv",
        "enable SeShutdownPrivilege: ok {ok}: {e} (1300 = not held)"
    );
    crate::log!("session", "locked {:?}", sys::session_locked());
    let (_, out) = sys::powershell(
        r#"Get-CimInstance Win32_Service | Where-Object { $_.DisplayName -match 'Standby|Lenovo' } | ForEach-Object { '{0} | {1} | {2} | {3}' -f $_.Name,$_.DisplayName,$_.State,$_.StartMode }"#,
    );
    for l in out.lines() {
        crate::log!("service", "{l}");
    }
    0
}

pub fn heat(args: &Args) -> i32 {
    let seconds = args.num("--seconds", 120);
    let threads = args.num("--load-threads", 0);
    let load_at = args.num("--load-at", 30);
    let load_for = args.num("--load-seconds", 60);
    let limit = args
        .num_opt("--heat-limit")
        .map_or(heat::DEFAULT_LIMIT_C, |c| c as f64);
    let t = match heat::Thermal::open() {
        Ok(t) => t,
        Err(e) => {
            crate::log!("heat", "{e}");
            return 1;
        }
    };
    let stop = Arc::new(AtomicBool::new(false));
    let mut workers = Vec::new();
    let start = Instant::now();
    let mut loaded = false;
    while start.elapsed().as_secs() < seconds {
        let s = start.elapsed().as_secs();
        if threads > 0 && !loaded && s >= load_at {
            crate::log!("heat", "load ON: {threads} threads for {load_for} s");
            for _ in 0..threads {
                let stop = stop.clone();
                workers.push(std::thread::spawn(move || {
                    let mut x = 1u64;
                    while !stop.load(Ordering::Relaxed) {
                        for _ in 0..100_000 {
                            x = std::hint::black_box(
                                x.wrapping_mul(6364136223846793005).wrapping_add(1),
                            );
                        }
                    }
                    x
                }));
            }
            loaded = true;
        }
        if loaded && !workers.is_empty() && s >= load_at + load_for {
            stop.store(true, Ordering::Relaxed);
            for w in workers.drain(..) {
                let _ = w.join();
            }
            crate::log!("heat", "load OFF");
        }
        match t.sample() {
            Ok(rs) => {
                let throttle: Vec<String> = rs
                    .iter()
                    .filter(|r| r.counter == "throttle_reasons")
                    .map(|r| format!("{}", r.value))
                    .collect();
                crate::log!(
                    "heat",
                    "t {s} s, load {}: {}, throttle reasons {}",
                    !workers.is_empty(),
                    heat::summary(&rs),
                    throttle.join("/")
                );
                if let Some(c) = heat::max_c(&rs)
                    && c >= limit
                    && !workers.is_empty()
                {
                    stop.store(true, Ordering::Relaxed);
                    for w in workers.drain(..) {
                        let _ = w.join();
                    }
                    crate::log!(
                        "heat",
                        "HEAT LIMIT: {c:.1} C >= {limit:.0} C, load OFF early"
                    );
                }
            }
            Err(e) => crate::log!("heat", "{e}"),
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    stop.store(true, Ordering::Relaxed);
    for w in workers.drain(..) {
        let _ = w.join();
    }
    0
}

fn log_wake_timer(t: &WakeTimer, after: u64) {
    crate::log!(
        "timer",
        "SetWaitableTimer(absolute +{after} s, resume TRUE) ok {}, last error {}{}",
        t.set_ok,
        t.last_error,
        if t.last_error == 50 {
            " = ERROR_NOT_SUPPORTED: Windows says this PC cannot be woken by the timer"
        } else {
            ""
        }
    );
}

pub fn timer_check(args: &Args) -> i32 {
    let after = args.num("--after", 20);
    if let Ok(g) = power::active_scheme() {
        for src in BOTH {
            let v = power::read(&g, power::WAKE_TIMERS, src).unwrap_or(99);
            crate::log!(
                "timer",
                "allow wake timers {}: {v} ({})",
                src.name(),
                power::wake_timers_name(v)
            );
        }
    }
    let t = match WakeTimer::arm(after) {
        Ok(t) => t,
        Err(e) => {
            crate::log!("timer", "{e}");
            return 1;
        }
    };
    let armed = Instant::now();
    log_wake_timer(&t, after);
    let (rc, out) = sys::run("powercfg", &["/waketimers"]);
    crate::log!("timer", "powercfg /waketimers rc {rc}:");
    for l in out.lines().filter(|l| !l.trim().is_empty()) {
        crate::log!("timer", "  {}", l.trim());
    }
    while armed.elapsed().as_secs() < after + 30 {
        if t.fired() {
            crate::log!(
                "timer",
                "fired after {:.3} s (due {after} s)",
                armed.elapsed().as_secs_f64()
            );
            return 0;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    crate::log!("timer", "did not fire within {} s", after + 30);
    1
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LidState {
    WaitClose,
    Working { until_ms: u64 },
    AfterWork,
    Opened { at_ms: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LidInput {
    Closed,
    Opened,
    Tick,
    /// The hottest thermal zone reached `--heat-limit` while the lid was closed.
    HeatLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LidAction {
    OnClose,
    /// Release the hold and restore the plan; with the lid still closed, also run the end method.
    WorkEnd {
        lid_open: bool,
    },
    Finish,
}

pub struct LidRules {
    pub work_ms: u64,
    /// `--end none`: hold until the lid opens (the docked check).
    pub hold_until_open: bool,
    pub after_open_ms: u64,
}

/// The lid test as a pure state machine, so its order of steps is tested without a lid.
pub fn lid_step(
    s: LidState,
    input: LidInput,
    now_ms: u64,
    r: &LidRules,
) -> (LidState, Option<LidAction>) {
    use LidAction::*;
    use LidInput::*;
    use LidState::*;
    match (s, input) {
        (WaitClose, Closed) => (
            Working {
                until_ms: now_ms + r.work_ms,
            },
            Some(OnClose),
        ),
        (Working { .. }, Opened) => (
            LidState::Opened { at_ms: now_ms },
            Some(WorkEnd { lid_open: true }),
        ),
        (Working { until_ms }, Tick) if !r.hold_until_open && now_ms >= until_ms => {
            (AfterWork, Some(WorkEnd { lid_open: false }))
        }
        // A hot closed laptop matters more than the test, so this also ends `--end none` holds.
        (Working { .. }, HeatLimit) => (AfterWork, Some(WorkEnd { lid_open: false })),
        (AfterWork, Opened) => (LidState::Opened { at_ms: now_ms }, None),
        (LidState::Opened { at_ms }, Tick) if now_ms >= at_ms + r.after_open_ms => {
            (s, Some(Finish))
        }
        _ => (s, None),
    }
}

/// The chime runs on its own thread; the lock waits until its stream has started (at most 1 s), so the sound
/// is already playing when the session locks.
fn play_chime_before_lock() {
    let (tx, rx) = std::sync::mpsc::channel();
    let t = Instant::now();
    let _ = sound::play_chime("lid", Some(tx));
    match rx.recv_timeout(Duration::from_millis(1000)) {
        Ok(()) => crate::log!(
            "lid",
            "chime running {} ms after the call",
            t.elapsed().as_millis()
        ),
        Err(_) => crate::log!("lid", "chime not started within 1 s: going on"),
    }
}

fn lock() {
    let t = Instant::now();
    let r = unsafe { LockWorkStation() };
    crate::log!(
        "lid",
        "LockWorkStation {r:?} in {} us, session locked now {:?}",
        t.elapsed().as_micros(),
        sys::session_locked()
    );
}

/// Runs a sleep method. SetSuspendState returns only after the PC wakes, if it slept.
/// Returns false when the call itself failed.
fn end_method(method: &str) -> bool {
    match method {
        "release" | "none" => {
            crate::log!("end", "nothing more: requests released, plan restored");
            true
        }
        "screen-off" => {
            let (r, e) = power::screen_off();
            crate::log!(
                "end",
                "SendMessageTimeout(HWND_BROADCAST, SC_MONITORPOWER, 2) result {r}, last error {e}"
            );
            r != 0
        }
        "sleep" | "hibernate" => {
            let (ok, e) = sys::enable_shutdown_privilege();
            crate::log!("end", "enable SeShutdownPrivilege: ok {ok}: {e}");
            let hib = method == "hibernate";
            crate::log!(
                "end",
                "calling SetSuspendState(hibernate {hib}, force false, wake events on)"
            );
            let (ok, e, ms) = power::set_suspend_state(hib);
            crate::log!(
                "end",
                "SetSuspendState returned {ok}, last error {e}{}, after {ms} ms",
                if e == 50 {
                    " (ERROR_NOT_SUPPORTED)"
                } else {
                    ""
                }
            );
            ok
        }
        other => {
            crate::log!("end", "unknown method {other}");
            false
        }
    }
}

pub fn lid(args: &Args) -> i32 {
    let end = args.opt("--end").unwrap_or_else(|| "release".into());
    if !matches!(
        end.as_str(),
        "release" | "screen-off" | "sleep" | "hibernate" | "none"
    ) {
        crate::log!("lid", "bad --end {end}");
        return 2;
    }
    let rules = LidRules {
        work_ms: args.num("--work", 60) * 1000,
        hold_until_open: end == "none",
        after_open_ms: args.num("--after-open", 20) * 1000,
    };
    let max_ms = args.num("--max-minutes", 12) * 60_000;
    let heat_limit = args
        .num_opt("--heat-limit")
        .map_or(heat::DEFAULT_LIMIT_C, |c| c as f64);
    let no_lock = args.flag("--no-lock");
    let no_sound = args.flag("--no-sound");
    let sound_kind = args.opt("--sound").unwrap_or_else(|| "chime".into());
    if !matches!(sound_kind.as_str(), "chime" | "alias") {
        crate::log!("lid", "bad --sound {sound_kind}");
        return 2;
    }
    let alias_after = args.num_opt("--alias-after");
    let mut alias_done = false;
    let chime_after = args.num_opt("--chime-after");
    let mut chime_done = false;
    let listener = args.flag("--listen").then(sound::Listener::start);
    if listener.is_some() {
        sound::read_state("audio");
    }
    if !no_sound {
        sound::warm_up("audio");
    }
    let simulate = args.num_opt("--simulate-close-after");
    let mut heat_stop: Option<f64> = None;
    let since = events_since_now();
    plan::log_values("before");
    let mut copy = match PlanCopy::make(
        "Wakeshot spike lid test",
        &[
            (power::LID, power::Source::Ac, 0),
            (power::LID, power::Source::Dc, 0),
        ],
    ) {
        Ok(c) => c,
        Err(e) => {
            crate::log!("lid", "cannot make plan copy: {e}");
            return 1;
        }
    };
    let subs = Subscriptions::start(&events::DEFAULT_SETTINGS, true);
    let mut hold = match Hold::request("Wakeshot lid spike: simulated work") {
        Ok(h) => Some(h),
        Err(e) => {
            crate::log!("lid", "{e}");
            None
        }
    };
    let mut ticker = Ticker::new(1000);
    let t0 = wall_ms();
    let mut state = LidState::WaitClose;
    let mut lid_closed: Option<bool> = None;
    let mut close_at: Option<Instant> = None;
    let mut marks = LidMarks::default();
    events::nap(600);
    for (_, ev) in events::drain() {
        if let Ev::Setting { name: "lid", value } = ev {
            lid_closed = Some(value == 0);
        }
    }
    crate::log!(
        "lid",
        "initial lid closed {lid_closed:?}. Work lasts {} s after the lid closes, then end method '{end}'. lock {}, sound {} ({sound_kind}), alias control after {alias_after:?} s, chime after {chime_after:?} s, listen {}, heat limit {heat_limit:.0} C, max {} min",
        rules.work_ms / 1000,
        !no_lock,
        !no_sound,
        listener.is_some(),
        max_ms / 60_000
    );
    let mut inputs: Vec<LidInput> = Vec::new();
    if lid_closed == Some(true) {
        crate::log!(
            "lid",
            "lid already closed at start: treating as a close now"
        );
        inputs.push(LidInput::Closed);
    }
    crate::log!("lid", ">>> READY: close the lid now <<<");
    let mut finished = false;
    while !finished {
        for (at, ev) in events::drain() {
            match ev {
                Ev::Setting { name: "lid", value } => {
                    lid_closed = Some(value == 0);
                    inputs.push(if value == 0 {
                        LidInput::Closed
                    } else {
                        LidInput::Opened
                    });
                }
                Ev::Setting { name, value } if name.ends_with("display") => {
                    if let Some(c) = close_at {
                        crate::log!(
                            "lid",
                            "display {} at {:.3} s after the lid close callback",
                            events::value_text(name, value),
                            at.saturating_duration_since(c).as_secs_f64()
                        );
                    }
                }
                Ev::Suspend(k) => crate::log!(
                    "lid",
                    "{} at t {} s",
                    events::suspend_name(k),
                    ticker.elapsed_s()
                ),
                _ => {}
            }
        }
        if let Some(s) = simulate
            && state == LidState::WaitClose
            && wall_ms() >= t0 + s * 1000
        {
            crate::log!("lid", "SIMULATED lid close (dry run)");
            inputs.push(LidInput::Closed);
        }
        inputs.push(LidInput::Tick);
        for input in inputs.drain(..) {
            let (next, action) = lid_step(state, input, wall_ms(), &rules);
            if next != state {
                crate::log!("lid", "state {state:?} -> {next:?} on {input:?}");
            }
            if let LidState::Opened { at_ms } = next {
                marks.open_ms.get_or_insert(at_ms);
            }
            state = next;
            match action {
                Some(LidAction::OnClose) => {
                    close_at = Some(Instant::now());
                    marks.close_ms = Some(wall_ms());
                    crate::log!("lid", "LID CLOSED: reacting now");
                    if !no_sound {
                        if sound_kind == "alias" {
                            sound::play_alias("lid");
                        } else {
                            play_chime_before_lock();
                        }
                    }
                    if !no_lock {
                        lock();
                    }
                }
                Some(LidAction::WorkEnd { lid_open }) => {
                    // Re-check the last lid callback right before acting.
                    let lid_open = lid_open || lid_closed == Some(false);
                    crate::log!("end", "WORK END, lid open {lid_open}");
                    marks.work_end_ms = Some(wall_ms());
                    marks.open_at_end = lid_open;
                    drop(hold.take());
                    copy.restore();
                    if lid_open {
                        crate::log!("end", "lid is open: no end method (re-check before acting)");
                    } else {
                        let _ = end_method(&end);
                    }
                }
                Some(LidAction::Finish) => {
                    crate::log!(
                        "lid",
                        "lid open for {} s: finishing",
                        rules.after_open_ms / 1000
                    );
                    finished = true;
                }
                None => {}
            }
        }
        if let (Some(s), Some(c), false) = (alias_after, close_at, alias_done)
            && c.elapsed() >= Duration::from_secs(s)
        {
            alias_done = true;
            crate::log!(
                "lid",
                "control: the Phase 0 alias sound, {} ms after the close",
                c.elapsed().as_millis()
            );
            sound::play_alias("lid");
        }
        if let (Some(s), Some(c), false) = (chime_after, close_at, chime_done)
            && c.elapsed() >= Duration::from_secs(s)
        {
            chime_done = true;
            crate::log!(
                "lid",
                "second sound: our chime, {} ms after the close",
                c.elapsed().as_millis()
            );
            let _ = sound::play_chime("lid", None);
        }
        let phase = match state {
            LidState::WaitClose => "wait-close",
            LidState::Working { .. } => "work",
            LidState::AfterWork => "after-work",
            LidState::Opened { .. } => "opened",
        };
        ticker.tick(phase);
        if let (LidState::Working { .. }, Some(c), None) = (state, ticker.heat_c, heat_stop)
            && c >= heat_limit
        {
            crate::log!(
                "lid",
                "HEAT LIMIT: {c:.1} C >= {heat_limit:.0} C, ending work now"
            );
            heat_stop = Some(c);
            inputs.push(LidInput::HeatLimit);
        }
        if wall_ms() >= t0 + max_ms {
            crate::log!("lid", "max time reached");
            break;
        }
        if !events::nap(1000) {
            crate::log!("lid", "stop requested");
            break;
        }
    }
    drop(hold.take());
    let restored = copy.restore();
    drop(subs);
    if let Some(l) = listener {
        l.stop();
    }
    for line in lid_result(&marks, &ticker.gaps) {
        crate::log!("result", "{line}");
    }
    if let Some(c) = heat_stop {
        crate::log!(
            "result",
            "RESULT: work ended early at the heat limit ({c:.1} C >= {heat_limit:.0} C)"
        );
    }
    finish(&since, &ticker, restored)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LidMarks {
    pub close_ms: Option<u64>,
    pub work_end_ms: Option<u64>,
    pub open_ms: Option<u64>,
    /// The re-check found the lid open, so no end method ran.
    pub open_at_end: bool,
}

/// Plain result lines for the person who ran the test. A gap is placed in a phase by its start;
/// 1.5 s of slack covers the tick that runs just before a sleep call.
pub fn lid_result(m: &LidMarks, gaps: &[GapInfo]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(close) = m.close_ms else {
        out.push("RESULT: the lid never closed".into());
        return out;
    };
    let work_end = m.work_end_ms.unwrap_or(u64::MAX);
    let during: Vec<&GapInfo> = gaps
        .iter()
        .filter(|g| g.start_ms + 1500 >= close && g.start_ms + 1500 < work_end)
        .collect();
    if during.is_empty() {
        out.push("RESULT: with the request held, the PC kept running with the lid closed".into());
    } else {
        out.push(format!(
            "RESULT: the PC STOPPED while the request was held and the lid closed: {} gap(s), first {:.0} s after close for {:.0} s",
            during.len(),
            (during[0].start_ms.saturating_sub(close)) as f64 / 1000.0,
            during[0].len_ms as f64 / 1000.0
        ));
    }
    match m.work_end_ms {
        None => out.push("RESULT: work did not end before the lid opened".into()),
        Some(end) => {
            let after = gaps.iter().find(|g| g.start_ms + 1500 >= end);
            let opened_before_end = m.open_at_end || m.open_ms.is_some_and(|o| o <= end);
            match after {
                _ if opened_before_end => {
                    out.push("RESULT: the lid was open when work ended, so no end method ran".into())
                }
                Some(g) => out.push(format!(
                    "RESULT: after work ended the PC stopped ({:?}) about {:.0} s later, for {:.0} s",
                    g.kind,
                    (g.start_ms.saturating_sub(end)) as f64 / 1000.0,
                    g.len_ms as f64 / 1000.0
                )),
                None => out.push(
                    "RESULT: after work ended the PC kept running until the lid opened (no standby)"
                        .into(),
                ),
            }
        }
    }
    out
}

fn finish(since: &str, ticker: &Ticker, restored: bool) -> i32 {
    plan::log_values("after");
    events::dump_power_events(since);
    crate::log!(
        "result",
        "gaps: {}",
        if ticker.gaps.is_empty() {
            "none".to_string()
        } else {
            ticker
                .gaps
                .iter()
                .map(|g| g.text.as_str())
                .collect::<Vec<_>>()
                .join(" / ")
        }
    );
    if restored {
        crate::log!("result", "DONE: plan restored. You can close this window.");
        0
    } else {
        crate::log!("result", "DONE but NOT RESTORED: run `restore`");
        1
    }
}

pub fn suspend(args: &Args) -> i32 {
    let Some(method) = args.opt("--method") else {
        crate::log!("suspend", "--method sleep|hibernate|screen-off is required");
        return 2;
    };
    if !matches!(method.as_str(), "sleep" | "hibernate" | "screen-off") {
        crate::log!("suspend", "bad --method {method}");
        return 2;
    }
    let dry = args.flag("--dry");
    let max_ms = args.num("--max-minutes", 6) * 60_000;
    let countdown = args.num("--countdown", 10);
    let wake_after = args.num("--wake-after", 0);
    let since = events_since_now();
    plan::log_values("before");
    let subs = Subscriptions::start(&events::DEFAULT_SETTINGS, true);
    let mut ticker = Ticker::new(1000);
    crate::log!(
        "suspend",
        ">>> In {countdown} s this program calls '{method}'. Do not touch the PC after that. <<<"
    );
    for _ in 0..countdown {
        ticker.tick("countdown");
        if !events::nap(1000) {
            drop(subs);
            return finish(&since, &ticker, true);
        }
    }
    let timer = if wake_after > 0 {
        match WakeTimer::arm(wake_after) {
            Ok(t) => {
                log_wake_timer(&t, wake_after);
                Some(t)
            }
            Err(e) => {
                crate::log!("timer", "{e}");
                None
            }
        }
    } else {
        None
    };
    let called = wall_ms();
    let mut failed = false;
    if dry {
        crate::log!("suspend", "dry run: not calling '{method}'");
        let (ok, e) = sys::enable_shutdown_privilege();
        crate::log!("suspend", "enable SeShutdownPrivilege: ok {ok}: {e}");
    } else {
        failed = !end_method(&method);
        if failed {
            crate::log!("suspend", "the call failed: finishing in 20 s");
        }
    }
    let mut back_at: Option<u64> = None;
    let mut timer = timer;
    loop {
        for (_, ev) in events::drain() {
            if let Ev::Suspend(k) = ev {
                crate::log!(
                    "suspend",
                    "{} at t {} s",
                    events::suspend_name(k),
                    ticker.elapsed_s()
                );
            }
        }
        if let Some(t) = &timer
            && t.fired()
        {
            crate::log!(
                "timer",
                "WAKE TIMER fired, {} s after the call (due {wake_after} s)",
                (wall_ms() - called) / 1000
            );
            timer = None;
        }
        ticker.tick("after-call");
        let now = wall_ms();
        // The user is back: input in the last 5 s, at least 15 s after the call.
        if back_at.is_none() && now >= called + 15_000 && sys::idle_ms() < 5000 {
            crate::log!("suspend", "input seen: finishing in 20 s");
            back_at = Some(now);
        }
        if back_at.is_some_and(|b| now >= b + 20_000) || ((dry || failed) && now >= called + 20_000)
        {
            break;
        }
        if now >= called + max_ms {
            crate::log!("suspend", "max time reached");
            break;
        }
        if !events::nap(1000) {
            break;
        }
    }
    drop(subs);
    let line = match ticker.gaps.iter().find(|g| g.start_ms + 1500 >= called) {
        Some(g) => format!(
            "RESULT: the PC stopped ({:?}) {:.0} s after the call, for {:.0} s",
            g.kind,
            (g.start_ms.saturating_sub(called)) as f64 / 1000.0,
            g.len_ms as f64 / 1000.0
        ),
        None => "RESULT: the PC did not sleep after the call".into(),
    };
    crate::log!("result", "{line}");
    finish(&since, &ticker, true)
}

/// Did the timer end a sleep? A gap that spans the due time and ends near it means yes.
pub fn wake_result(armed_ms: u64, after_s: u64, gaps: &[GapInfo]) -> String {
    let due = armed_ms + after_s * 1000;
    let spanning = gaps
        .iter()
        .find(|g| g.start_ms <= due && g.start_ms + g.len_ms as u64 + 5000 >= due);
    match spanning {
        None => "RESULT: the PC was not asleep when the timer was due, so this run proves nothing"
            .into(),
        Some(g) => {
            let end = g.start_ms + g.len_ms as u64;
            let late = (end as i64 - due as i64) as f64 / 1000.0;
            if late <= 30.0 {
                format!(
                    "RESULT: the timer woke the PC (it ran again {late:.0} s after the due time)"
                )
            } else {
                format!(
                    "RESULT: the PC slept and the timer did NOT wake it (it ran again only {late:.0} s after the due time)"
                )
            }
        }
    }
}

pub fn wake(args: &Args) -> i32 {
    let after = args.num("--after", 120);
    let max_ms = args.num("--max-minutes", 6) * 60_000;
    let since = events_since_now();
    plan::log_values("before");
    let mut copy = match args.num_opt("--wake-timers") {
        Some(v) => match PlanCopy::make(
            "Wakeshot spike wake test",
            &[
                (power::WAKE_TIMERS, power::Source::Ac, v as u32),
                (power::WAKE_TIMERS, power::Source::Dc, v as u32),
            ],
        ) {
            Ok(c) => Some(c),
            Err(e) => {
                crate::log!("wake", "cannot make plan copy: {e}");
                return 1;
            }
        },
        None => None,
    };
    if let Ok(g) = power::active_scheme() {
        for src in BOTH {
            let v = power::read(&g, power::WAKE_TIMERS, src).unwrap_or(99);
            crate::log!(
                "wake",
                "allow wake timers {} on the active plan: {v} ({})",
                src.name(),
                power::wake_timers_name(v)
            );
        }
    }
    let subs = Subscriptions::start(&events::DEFAULT_SETTINGS, true);
    let mut ticker = Ticker::new(1000);
    let t = match WakeTimer::arm(after) {
        Ok(t) => t,
        Err(e) => {
            crate::log!("wake", "{e}");
            return 1;
        }
    };
    log_wake_timer(&t, after);
    let armed = wall_ms();
    crate::log!(
        "wake",
        ">>> Now put the PC to sleep: Start > Power > Sleep. Then do not touch it. The timer is due at +{after} s. <<<"
    );
    let mut fired_at: Option<u64> = None;
    let mut back_at: Option<u64> = None;
    loop {
        for (_, ev) in events::drain() {
            if let Ev::Suspend(k) = ev {
                crate::log!(
                    "wake",
                    "{} at +{} s",
                    events::suspend_name(k),
                    (wall_ms() - armed) / 1000
                );
            }
        }
        let now = wall_ms();
        if fired_at.is_none() && t.fired() {
            fired_at = Some(now);
            crate::log!(
                "wake",
                "TIMER FIRED: seen at +{:.1} s, due +{after} s, late {:.1} s",
                (now - armed) as f64 / 1000.0,
                (now as f64 - armed as f64) / 1000.0 - after as f64
            );
        }
        ticker.tick(if fired_at.is_some() {
            "after-fire"
        } else {
            "armed"
        });
        // Finish when the user is back after the timer was due.
        if back_at.is_none() && now >= armed + (after + 20) * 1000 && sys::idle_ms() < 5000 {
            crate::log!("wake", "input seen: finishing in 15 s");
            back_at = Some(now);
        }
        if back_at.is_some_and(|b| now >= b + 15_000) || now >= armed + max_ms {
            break;
        }
        if !events::nap(1000) {
            break;
        }
    }
    drop(t);
    drop(subs);
    crate::log!("result", "{}", wake_result(armed, after, &ticker.gaps));
    let restored = copy.as_mut().map(|c| c.restore()).unwrap_or(true);
    crate::log!(
        "result",
        "timer {}",
        match fired_at {
            Some(f) => format!("fired, seen at +{} s", (f - armed) / 1000),
            None => "never fired".into(),
        }
    );
    finish(&since, &ticker, restored)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResumePhase {
    BeforeSleep,
    Hold { until_ms: u64 },
    Control { until_ms: u64 },
}

pub fn resume(args: &Args) -> i32 {
    let retake = args.flag("--retake");
    let kind = args.opt("--request").unwrap_or_else(|| "power".into());
    let idle = args.num("--idle", 60) as u32;
    let hold_ms = args.num("--hold-minutes", 4) * 60_000;
    let control_ms = args.num("--control-minutes", 3) * 60_000;
    let max_ms = args.num("--max-minutes", 20) * 60_000;
    let simulate = args.num_opt("--simulate-resume-after");
    let since = events_since_now();
    plan::log_values("before");
    let mut writes = Vec::new();
    if idle > 0 {
        for src in BOTH {
            writes.push((power::SLEEP, src, idle));
            writes.push((power::DISPLAY_OFF, src, idle));
        }
    }
    let mut copy = match PlanCopy::make("Wakeshot spike resume test", &writes) {
        Ok(c) => c,
        Err(e) => {
            crate::log!("resume", "cannot make plan copy: {e}");
            return 1;
        }
    };
    let subs = Subscriptions::start(&events::DEFAULT_SETTINGS, true);
    let mut hold = Some(if kind == "stes" {
        Hold::stes()
    } else {
        match Hold::request("Wakeshot lid spike: resume test") {
            Ok(h) => h,
            Err(e) => {
                crate::log!("resume", "{e}");
                return 1;
            }
        }
    });
    let mut ticker = Ticker::new(1000);
    let t0 = wall_ms();
    let mut phase = ResumePhase::BeforeSleep;
    crate::log!(
        "resume",
        ">>> Now: Start > Power > Sleep. Wait 20 s. Press the power button and sign in. Then do not touch the PC until it prints DONE. <<< (request '{kind}', retake {retake}, sleep and display off {idle} s on the copy)"
    );
    let mut hold_gaps = 0;
    let mut control_gaps = 0;
    loop {
        let mut resumed = false;
        for (_, ev) in events::drain() {
            if let Ev::Suspend(k) = ev {
                crate::log!("resume", "{} in phase {phase:?}", events::suspend_name(k));
                if k == 18 || k == 7 {
                    resumed = true;
                }
            }
        }
        if let Some(s) = simulate
            && phase == ResumePhase::BeforeSleep
            && wall_ms() >= t0 + s * 1000
        {
            crate::log!("resume", "SIMULATED resume (dry run)");
            resumed = true;
        }
        let name = match phase {
            ResumePhase::BeforeSleep => "before-sleep",
            ResumePhase::Hold { .. } => "hold",
            ResumePhase::Control { .. } => "control",
        };
        let gap = ticker.tick(name);
        let now = wall_ms();
        match phase {
            ResumePhase::BeforeSleep => {
                // The first gap is the manual sleep itself.
                if resumed || gap.is_some() {
                    crate::log!(
                        "resume",
                        "RESUMED: hold phase for {} s starts",
                        hold_ms / 1000
                    );
                    if retake && let Some(h) = &hold {
                        h.retake();
                    }
                    phase = ResumePhase::Hold {
                        until_ms: now + hold_ms,
                    };
                }
            }
            ResumePhase::Hold { until_ms } => {
                if gap.is_some() {
                    hold_gaps += 1;
                    crate::log!(
                        "resume",
                        "REQUEST NOT HONORED: the PC slept or paused while held"
                    );
                }
                if now >= until_ms {
                    drop(hold.take());
                    crate::log!(
                        "resume",
                        "control phase for {} s: request released, the PC should sleep about {idle} s after the last input",
                        control_ms / 1000
                    );
                    phase = ResumePhase::Control {
                        until_ms: now + control_ms,
                    };
                }
            }
            ResumePhase::Control { until_ms } => {
                if gap.is_some() {
                    control_gaps += 1;
                    crate::log!(
                        "resume",
                        "control: the PC slept without the request (expected)"
                    );
                }
                if now >= until_ms {
                    break;
                }
            }
        }
        if now >= t0 + max_ms {
            crate::log!("resume", "max time reached");
            break;
        }
        if !events::nap(1000) {
            break;
        }
    }
    drop(hold.take());
    drop(subs);
    let restored = copy.restore();
    crate::log!(
        "result",
        "hold-phase gaps {hold_gaps} ({}), control-phase gaps {control_gaps} ({})",
        if hold_gaps == 0 {
            "request honored"
        } else {
            "request NOT honored"
        },
        if control_gaps > 0 {
            "control slept: the test is valid"
        } else {
            "control did not sleep: the hold result proves nothing"
        }
    );
    finish(&since, &ticker, restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticks::Gap;

    const R: LidRules = LidRules {
        work_ms: 60_000,
        hold_until_open: false,
        after_open_ms: 20_000,
    };

    #[test]
    fn close_starts_work_and_reacts() {
        let (s, a) = lid_step(LidState::WaitClose, LidInput::Closed, 1000, &R);
        assert_eq!(s, LidState::Working { until_ms: 61_000 });
        assert_eq!(a, Some(LidAction::OnClose));
    }

    #[test]
    fn work_ends_on_time_with_lid_closed() {
        let w = LidState::Working { until_ms: 61_000 };
        assert_eq!(lid_step(w, LidInput::Tick, 60_999, &R), (w, None));
        assert_eq!(
            lid_step(w, LidInput::Tick, 61_000, &R),
            (
                LidState::AfterWork,
                Some(LidAction::WorkEnd { lid_open: false })
            )
        );
    }

    #[test]
    fn opening_early_ends_work_without_the_end_method() {
        let w = LidState::Working { until_ms: 61_000 };
        assert_eq!(
            lid_step(w, LidInput::Opened, 30_000, &R),
            (
                LidState::Opened { at_ms: 30_000 },
                Some(LidAction::WorkEnd { lid_open: true })
            )
        );
    }

    #[test]
    fn docked_check_holds_until_open() {
        let r = LidRules {
            hold_until_open: true,
            ..R
        };
        let w = LidState::Working { until_ms: 61_000 };
        assert_eq!(lid_step(w, LidInput::Tick, 999_999, &r), (w, None));
    }

    #[test]
    fn finishes_after_open_delay() {
        let o = LidState::Opened { at_ms: 100_000 };
        assert_eq!(lid_step(o, LidInput::Tick, 119_999, &R), (o, None));
        assert_eq!(
            lid_step(o, LidInput::Tick, 120_000, &R),
            (o, Some(LidAction::Finish))
        );
        let (s, a) = lid_step(LidState::AfterWork, LidInput::Opened, 5, &R);
        assert_eq!((s, a), (LidState::Opened { at_ms: 5 }, None));
    }

    #[test]
    fn heat_limit_ends_work_even_in_the_docked_check() {
        let w = LidState::Working { until_ms: 61_000 };
        let end = (
            LidState::AfterWork,
            Some(LidAction::WorkEnd { lid_open: false }),
        );
        assert_eq!(lid_step(w, LidInput::HeatLimit, 10_000, &R), end);
        let r = LidRules {
            hold_until_open: true,
            ..R
        };
        assert_eq!(lid_step(w, LidInput::HeatLimit, 10_000, &r), end);
    }

    #[test]
    fn heat_limit_outside_work_does_nothing() {
        for s in [
            LidState::WaitClose,
            LidState::AfterWork,
            LidState::Opened { at_ms: 5 },
        ] {
            assert_eq!(lid_step(s, LidInput::HeatLimit, 10, &R), (s, None));
        }
    }

    #[test]
    fn open_while_waiting_is_ignored() {
        assert_eq!(
            lid_step(LidState::WaitClose, LidInput::Opened, 5, &R),
            (LidState::WaitClose, None)
        );
    }

    fn gap(start_ms: u64, len_ms: i64) -> GapInfo {
        GapInfo {
            kind: Gap::Paused,
            start_ms,
            len_ms,
            text: String::new(),
        }
    }

    #[test]
    fn lid_result_kept_running_then_slept() {
        let m = LidMarks {
            close_ms: Some(10_000),
            work_end_ms: Some(70_000),
            open_ms: Some(300_000),
            open_at_end: false,
        };
        let r = lid_result(&m, &[gap(75_000, 200_000)]);
        assert!(r[0].contains("kept running with the lid closed"), "{r:?}");
        assert!(
            r[1].contains("stopped (Paused) about 5 s later, for 200 s"),
            "{r:?}"
        );
    }

    #[test]
    fn lid_result_stopped_during_work() {
        let m = LidMarks {
            close_ms: Some(10_000),
            work_end_ms: Some(70_000),
            open_ms: None,
            open_at_end: false,
        };
        let r = lid_result(&m, &[gap(20_000, 30_000)]);
        assert!(r[0].contains("STOPPED"), "{r:?}");
        assert!(r[1].contains("kept running until the lid opened"), "{r:?}");
    }

    #[test]
    fn lid_result_sleep_call_gap_counts_after_work() {
        // The last tick ran 0.8 s before the sleep call at work end.
        let m = LidMarks {
            close_ms: Some(10_000),
            work_end_ms: Some(70_000),
            open_ms: Some(200_000),
            open_at_end: false,
        };
        let r = lid_result(&m, &[gap(69_200, 100_000)]);
        assert!(r[0].contains("kept running"), "{r:?}");
        assert!(r[1].contains("stopped"), "{r:?}");
    }

    #[test]
    fn wake_results() {
        // Armed at 0, due at 120 s.
        assert!(wake_result(0, 120, &[gap(20_000, 102_000)]).contains("woke the PC"));
        assert!(wake_result(0, 120, &[gap(20_000, 240_000)]).contains("did NOT wake"));
        assert!(wake_result(0, 120, &[gap(20_000, 30_000)]).contains("proves nothing"));
        assert!(wake_result(0, 120, &[]).contains("proves nothing"));
    }

    #[test]
    fn lid_result_open_at_end() {
        let m = LidMarks {
            close_ms: Some(10_000),
            work_end_ms: Some(70_000),
            open_ms: None,
            open_at_end: true,
        };
        assert!(lid_result(&m, &[])[1].contains("no end method ran"));
    }

    #[test]
    fn lid_result_without_close() {
        assert_eq!(
            lid_result(&LidMarks::default(), &[]),
            vec!["RESULT: the lid never closed".to_string()]
        );
    }

    #[test]
    fn events_query_starts_at_the_minute() {
        let s = events_since_now();
        assert_eq!(s.len(), 19);
        assert!(s.ends_with(":00"));
    }
}
