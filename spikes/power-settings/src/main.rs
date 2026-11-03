mod ctrl;
mod journal;
mod log;
mod power;
mod sys;
mod watch;

use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};

use windows::Win32::System::Power::*;
use windows::Win32::System::Registry::{KEY_READ, KEY_WRITE};
use windows::Win32::System::SystemServices::POWER_REQUEST_CONTEXT_VERSION;
use windows::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};
use windows::core::{GUID, PWSTR};

use journal::{Action, Journal};
use power::{LID, SLEEP, Source, guid_str};
use watch::{Detectors, Options};

const USAGE: &str = "usage: power-settings-spike <mode> [options]
modes:
  read                      print every value the card asks about (no writes)
  write-test                write lid action and sleep timeout on the active plan, then restore
  copy-test [--keep]        duplicate the plan, make the copy active, run change detection, remove it
  watch --seconds N         log every change to the watched values for N seconds
  hold [--sleep-dc S] [--minutes M] [--request system+execution|system|stes|none]
                            battery rule test on a temporary plan copy (needs battery)
  restore                   undo whatever out/journal.txt says the spike changed
  set-overlay <guid>        put a Power mode overlay back (undocumented API)
  limited <mode> [options]  run a mode with Explorer's unelevated token and wait for it
options: --log <file>, --allow-elevated, --allow-ac";

fn out_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("out")
}

fn journal_path() -> PathBuf {
    out_dir().join("journal.txt")
}

/// Every spike copy, from either spike, is named so.
const SPIKE_PREFIX: &str = "Wakeshot spike";

/// The lid spike's journal. While it exists, the active plan may be that spike's copy.
fn lid_journal_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("lid-sleep")
        .join("out")
        .join("journal.txt")
}

fn is_spike_plan(g: &GUID) -> bool {
    power::friendly_name(g).starts_with(SPIKE_PREFIX)
}

/// Plans that exist now and are not spike copies: the only ones restore may switch to.
fn usable_plans() -> Vec<GUID> {
    power::schemes()
        .into_iter()
        .filter(|g| !is_spike_plan(g))
        .collect()
}

/// A test that starts on top of the other spike's copy would copy its test values, and restore
/// would put that copy back instead of the user's plan.
fn other_spike_left() -> Option<String> {
    if lid_journal_path().exists() {
        return Some(format!(
            "the lid spike's journal {} exists. Run its `restore` first.",
            lid_journal_path().display()
        ));
    }
    let active = power::active_scheme().ok()?;
    is_spike_plan(&active).then(|| {
        format!(
            "the active plan {} ({}) is a spike copy. Run both restores first.",
            guid_str(&active),
            power::friendly_name(&active)
        )
    })
}

struct Args(Vec<String>);

impl Args {
    fn flag(&self, name: &str) -> bool {
        self.0.iter().any(|a| a == name)
    }
    fn opt(&self, name: &str) -> Option<String> {
        let i = self.0.iter().position(|a| a == name)?;
        self.0.get(i + 1).cloned()
    }
    fn num(&self, name: &str, default: u64) -> u64 {
        self.opt(name)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
}

fn stamp() -> String {
    log::wall_clock()
        .replace(['-', ':'], "")
        .replace(' ', "-")
        .split('.')
        .next()
        .unwrap()
        .to_string()
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let Some(mode) = raw.first().cloned() else {
        println!("{USAGE}");
        std::process::exit(2);
    };
    let args = Args(raw.clone());
    if mode == "limited" {
        std::process::exit(limited(&raw[1..]));
    }
    let log_path = args
        .opt("--log")
        .map(PathBuf::from)
        .unwrap_or_else(|| out_dir().join(format!("{mode}-{}.log", stamp())));
    log::init(Some(&log_path));
    let tok = sys::token_info();
    let ps = sys::power_status();
    log!(
        "start",
        "mode {mode}, pid {}, elevated {}, integrity {} (0x{:x}), on AC {}, battery {}%, energy saver on {}, idle {} s",
        std::process::id(),
        tok.elevated,
        tok.integrity_name(),
        tok.integrity_rid,
        ps.on_ac,
        ps.battery_percent,
        ps.energy_saver_on,
        sys::idle_ms() / 1000
    );
    let writes = matches!(mode.as_str(), "write-test" | "copy-test" | "hold");
    if writes && tok.elevated && !args.flag("--allow-elevated") {
        log!(
            "start",
            "refusing: elevated token. Wakeshot runs as a normal user. Open C:\\Users\\haika\\code\\wakeshot\\spikes\\morning-shell.cmd from File Explorer and run it there."
        );
        std::process::exit(3);
    }
    if writes
        && mode != "hold"
        && let Ok(Some(_)) = journal::load(&journal_path())
    {
        log!(
            "start",
            "refusing: out/journal.txt exists. Run `restore` first."
        );
        std::process::exit(4);
    }
    if writes {
        if let Some(why) = other_spike_left() {
            log!("start", "refusing: {why}");
            std::process::exit(4);
        }
        // write-test is not in Haik's steps and keeps the old behavior: Ctrl+C kills it, and
        // `restore` undoes it from the journal.
        if mode != "write-test" {
            ctrl::install();
        }
    }
    // Ctrl+C ends a watch early and still prints which values differ from the start.
    if mode == "watch" {
        ctrl::install();
    }
    let code = match mode.as_str() {
        "read" => read(),
        "write-test" => write_test(),
        "copy-test" => copy_test(args.flag("--keep")),
        "watch" => watch_mode(args.num("--seconds", 600)),
        "hold" => hold(&args),
        "restore" => restore(),
        "set-overlay" => match args.0.get(1).and_then(|g| power::parse_guid(g)) {
            Some(g) => {
                log!(
                    "overlay",
                    "PowerSetActiveOverlayScheme({}) {}",
                    guid_str(&g),
                    power::set_overlay(g)
                );
                log!(
                    "overlay",
                    "actual now {}",
                    power::overlay("PowerGetActualOverlayScheme")
                );
                0
            }
            None => 2,
        },
        _ => {
            println!("{USAGE}");
            2
        }
    };
    ctrl::mark_clean();
    log!("end", "exit code {code}, log {}", log_path.display());
    std::process::exit(code);
}

fn limited(rest: &[String]) -> i32 {
    let Some(mode) = rest.first() else {
        println!("{USAGE}");
        return 2;
    };
    log::init(None);
    let mut args: Vec<String> = rest.to_vec();
    let log_path = match Args(args.clone()).opt("--log") {
        Some(p) => PathBuf::from(p),
        None => {
            let p = out_dir().join(format!("{mode}-limited-{}.log", stamp()));
            args.push("--log".into());
            args.push(p.display().to_string());
            p
        }
    };
    let exe = std::env::current_exe().unwrap().display().to_string();
    let quoted: Vec<String> = args
        .iter()
        .map(|a| {
            if a.contains(' ') {
                format!("\"{a}\"")
            } else {
                a.clone()
            }
        })
        .collect();
    let cwd = env!("CARGO_MANIFEST_DIR");
    let code = match sys::run_as_explorer_child(&exe, &quoted.join(" "), cwd) {
        Ok(c) => c as i32,
        Err(e) => {
            println!("limited: {e}");
            return 5;
        }
    };
    println!(
        "limited: child exit code {code}, log {}",
        log_path.display()
    );
    if let Ok(text) = std::fs::read_to_string(&log_path) {
        print!("{text}");
    }
    code
}

fn powercfg_current(lines_for: &[(&str, &str, &str, bool)]) {
    for (scheme, sub, setting, hidden) in lines_for {
        let q = if *hidden { "/qh" } else { "/q" };
        let (rc, out) = sys::run("powercfg", &[q, scheme, sub, setting]);
        let current: Vec<&str> = out
            .lines()
            .filter(|l| l.contains("Current"))
            .map(str::trim)
            .collect();
        log!(
            "powercfg",
            "{q} {scheme} {sub} {setting} rc {rc}: {}",
            current.join(" | ")
        );
    }
}

fn powercfg_lid_and_sleep() {
    powercfg_current(&[
        ("SCHEME_CURRENT", "SUB_BUTTONS", "LIDACTION", true),
        ("SCHEME_CURRENT", "SUB_SLEEP", "STANDBYIDLE", false),
    ]);
}

fn read() -> i32 {
    let active = match power::active_scheme() {
        Ok(g) => g,
        Err(rc) => {
            log!("read", "PowerGetActiveScheme rc {rc}");
            return 1;
        }
    };
    log!(
        "read",
        "active scheme {} \"{}\", built-in Balanced: {}",
        guid_str(&active),
        power::friendly_name(&active),
        active == power::SCHEME_BALANCED
    );
    for s in power::schemes() {
        log!(
            "read",
            "scheme {} \"{}\"",
            guid_str(&s),
            power::friendly_name(&s)
        );
    }
    for st in [
        LID,
        SLEEP,
        power::ENERGY_SAVER_THRESHOLD,
        power::SIGN_IN_AFTER_SLEEP,
        power::DISPLAY_OFF,
    ] {
        for src in [Source::Ac, Source::Dc] {
            let v = power::read(&active, st, src);
            let r = power::access_check_setting(st, src, false);
            let w = power::access_check_setting(st, src, true);
            log!(
                "read",
                "{}.{} = {v:?}; PowerSettingAccessCheckEx read {}, write {}",
                st.name,
                src.name(),
                power::err_text(r),
                power::err_text(w)
            );
        }
    }
    for (name, acc) in [
        ("ACCESS_ACTIVE_SCHEME", ACCESS_ACTIVE_SCHEME),
        ("ACCESS_CREATE_SCHEME", ACCESS_CREATE_SCHEME),
        ("ACCESS_SCHEME", ACCESS_SCHEME),
        ("ACCESS_ACTIVE_OVERLAY_SCHEME", ACCESS_ACTIVE_OVERLAY_SCHEME),
    ] {
        log!(
            "read",
            "PowerSettingAccessCheckEx({name}, null) read {}, write {}",
            power::err_text(power::access_check(acc, None, KEY_READ)),
            power::err_text(power::access_check(acc, None, KEY_WRITE))
        );
    }
    for (k, v) in watch::snapshot(true) {
        log!("read", "snapshot {k} = {v}");
    }
    let det = Detectors::start(Options {
        notify: vec![
            power::ESBATTTHRESHOLD,
            power::CONSOLELOCK,
            power::GUID_ACTIVE_POWERSCHEME,
            power::GUID_ACDC_POWER_SOURCE,
            power::GUID_ENERGY_SAVER_STATUS,
            power::GUID_CONSOLE_DISPLAY_STATE,
        ],
        registry: false,
        poll_ms: None,
        full: false,
        effective_mode: true,
        suspend_resume: false,
    });
    sleep(Duration::from_millis(1500));
    det.stop();
    log!("read", "idle (GetLastInputInfo) {} ms", sys::idle_ms());
    0
}

fn mark(name: &str) -> Instant {
    log!("trigger", ">>> {name}");
    Instant::now()
}

fn report(name: &str, t0: Instant, t1: Instant, rows: &mut Vec<String>) {
    let now = Instant::now();
    let first = watch::first_per_source(t0, now);
    let cell = |src: &str| {
        first
            .get(src)
            .map(|(ms, _)| format!("{ms} ms"))
            .unwrap_or_else(|| "none".into())
    };
    let row = format!(
        "| {name} | {} ms | {} | {} | {} |",
        (t1 - t0).as_millis(),
        cell("notify"),
        cell("reg-hklm"),
        cell("poll")
    );
    log!("result", "{row}");
    rows.push(row);
}

fn apply(actions: &[Action]) -> bool {
    let mut ok = true;
    for a in actions {
        match a {
            Action::WriteBack {
                scheme,
                setting,
                source,
                value,
            } => {
                let rc = power::write(scheme, *setting, *source, *value);
                log!(
                    "restore",
                    "write {} {}.{} = {value}: rc {}",
                    guid_str(scheme),
                    setting.name,
                    source.name(),
                    power::err_text(rc)
                );
                ok &= rc == 0;
            }
            Action::KeepUsers {
                scheme,
                setting,
                source,
                current,
            } => log!(
                "restore",
                "keep {} {}.{} = {current}: no longer our value",
                guid_str(scheme),
                setting.name,
                source.name()
            ),
            Action::SetActive(g) => {
                let rc = power::set_active(g);
                log!(
                    "restore",
                    "PowerSetActiveScheme({}) rc {}",
                    guid_str(g),
                    power::err_text(rc)
                );
                ok &= rc == 0;
            }
            Action::DeleteCopy(g) => {
                let rc = power::delete(g);
                log!(
                    "restore",
                    "PowerDeleteScheme({}) rc {}",
                    guid_str(g),
                    power::err_text(rc)
                );
                ok &= rc == 0 || rc == 2;
            }
        }
    }
    ok
}

fn restore_from(j: &Journal) -> bool {
    let active = power::active_scheme().unwrap_or(GUID::zeroed());
    let schemes = power::schemes();
    let plan = j.plan(active, &usable_plans(), |v| {
        if !schemes.contains(&v.scheme) {
            return None;
        }
        power::read(&v.scheme, v.setting, v.source).ok()
    });
    let ok = apply(&plan);
    let mut verified = true;
    for v in &j.values {
        if let Ok(now) = power::read(&v.scheme, v.setting, v.source) {
            let good = now == v.original || now != v.written;
            log!(
                "verify",
                "{} {}.{} = {now} (original {}, written {}) {}",
                guid_str(&v.scheme),
                v.setting.name,
                v.source.name(),
                v.original,
                v.written,
                if good { "ok" } else { "NOT RESTORED" }
            );
            verified &= good;
        }
    }
    let left: Vec<GUID> = power::schemes()
        .into_iter()
        .filter(|s| j.copies.contains(s))
        .collect();
    for c in &left {
        log!("verify", "copy {} still exists", guid_str(c));
    }
    ok && verified && left.is_empty()
}

fn restore() -> i32 {
    match journal::load(&journal_path()) {
        Ok(None) => {
            log!("restore", "no journal: nothing to restore");
            0
        }
        Ok(Some(j)) => {
            if restore_from(&j) {
                let _ = std::fs::remove_file(journal_path());
                log!("restore", "done, journal removed");
                0
            } else {
                log!("restore", "FAILED, journal kept");
                1
            }
        }
        Err(e) => {
            log!("restore", "bad journal: {e}");
            1
        }
    }
}

/// Never shorter than the user's value; 0 means Never and stays Never.
fn longer(v: u32) -> u32 {
    if v == 0 { 0 } else { v + 7 }
}

fn write_test() -> i32 {
    let scheme = power::active_scheme().expect("active scheme");
    log!(
        "write",
        "active scheme {} \"{}\"",
        guid_str(&scheme),
        power::friendly_name(&scheme)
    );
    powercfg_lid_and_sleep();
    let det = Detectors::start(Options {
        notify: vec![
            power::LIDACTION,
            power::STANDBYIDLE,
            power::GUID_ACTIVE_POWERSCHEME,
        ],
        registry: true,
        poll_ms: Some(250),
        full: false,
        effective_mode: false,
        suspend_resume: false,
    });
    sleep(Duration::from_millis(1500));
    let mut j = Journal {
        active_before: Some(scheme),
        ..Default::default()
    };
    let mut steps = Vec::new();
    for (st, src) in [
        (LID, Source::Ac),
        (LID, Source::Dc),
        (SLEEP, Source::Ac),
        (SLEEP, Source::Dc),
    ] {
        let original = power::read(&scheme, st, src).expect("read");
        let value = if st == LID { 0 } else { longer(original) };
        let r = power::access_check_setting(st, src, true);
        log!(
            "write",
            "{}.{} original {original}, will write {value}; PowerSettingAccessCheckEx write {}",
            st.name,
            src.name(),
            power::err_text(r)
        );
        j.record(scheme, st, src, original, value);
        steps.push((st, src, value));
    }
    journal::save(&journal_path(), &j);
    let mut rows = Vec::new();
    for (st, src, value) in steps {
        let name = format!("API write {}.{} = {value}", st.name, src.name());
        let t0 = mark(&name);
        let rc = power::write(&scheme, st, src, value);
        let t1 = Instant::now();
        log!("write", "{name}: rc {}", power::err_text(rc));
        sleep(Duration::from_millis(2500));
        report(&format!("{name}, no SetActive"), t0, t1, &mut rows);
        let t0 = mark("PowerSetActiveScheme(active)");
        let rc = power::set_active(&scheme);
        let t1 = Instant::now();
        log!("write", "PowerSetActiveScheme rc {}", power::err_text(rc));
        sleep(Duration::from_millis(2500));
        report("then PowerSetActiveScheme", t0, t1, &mut rows);
        let back = power::read(&scheme, st, src);
        log!("write", "read back {}.{} = {back:?}", st.name, src.name());
    }
    powercfg_lid_and_sleep();
    let t0 = mark("restore originals");
    let ok = restore_from(&j);
    let t1 = Instant::now();
    sleep(Duration::from_millis(2500));
    report("restore (writes + SetActive)", t0, t1, &mut rows);
    powercfg_lid_and_sleep();
    det.stop();
    log!(
        "result",
        "| trigger | call time | setting notification | registry notification | poll 250 ms |"
    );
    for r in &rows {
        log!("result", "{r}");
    }
    if ok {
        let _ = std::fs::remove_file(journal_path());
        log!("write", "restored and verified; journal removed");
        0
    } else {
        log!("write", "RESTORE FAILED; journal kept, run `restore`");
        1
    }
}

/// Every write names the copy, never "the active plan": if something switched plans meanwhile,
/// a write to the active plan would land on the user's own plan, which the journal cannot undo.
fn detect_sequence(scheme: &GUID, rows: &mut Vec<String>) {
    let copy = *scheme;
    let s = guid_str(scheme);
    let base = power::read(scheme, SLEEP, Source::Ac).expect("read sleep.ac");
    let lid_dc = power::read(scheme, LID, Source::Dc).expect("read lid.dc");
    let lid_ac = power::read(scheme, LID, Source::Ac).expect("read lid.ac");
    if base == 0 {
        log!("detect", "sleep.ac is Never; skipping the timer triggers");
        return;
    }
    let minutes = base / 60 + 1;
    let steps: Vec<(String, Box<dyn Fn() -> String>)> = vec![
        (
            format!("API write sleep.ac = {}, no SetActive", base + 7),
            Box::new(move || power::err_text(power::write(&copy, SLEEP, Source::Ac, base + 7))),
        ),
        (
            "API PowerSetActiveScheme".into(),
            Box::new(move || power::err_text(power::set_active(&copy))),
        ),
        (
            format!(
                "powercfg /setacvalueindex sleep {}, no /setactive",
                base + 14
            ),
            {
                let s = s.clone();
                Box::new(move || {
                    let v = (base + 14).to_string();
                    format!(
                        "{:?}",
                        sys::run(
                            "powercfg",
                            &["/setacvalueindex", &s, "SUB_SLEEP", "STANDBYIDLE", &v]
                        )
                    )
                })
            },
        ),
        ("powercfg /setactive".into(), {
            let s = s.clone();
            Box::new(move || format!("{:?}", sys::run("powercfg", &["/setactive", &s])))
        }),
        (
            format!("powercfg /change standby-timeout-ac {minutes}"),
            // `/change` has no plan argument: it always writes the active plan.
            Box::new(move || {
                if power::active_scheme().ok() != Some(copy) {
                    return "skipped: the active plan is not the copy".into();
                }
                let m = minutes.to_string();
                format!(
                    "{:?}",
                    sys::run("powercfg", &["/change", "standby-timeout-ac", &m])
                )
            }),
        ),
        (
            "powercfg /setdcvalueindex lid 0 + /setactive (DC value, on AC now)".into(),
            {
                let s = s.clone();
                Box::new(move || {
                    let a = sys::run(
                        "powercfg",
                        &["/setdcvalueindex", &s, "SUB_BUTTONS", "LIDACTION", "0"],
                    );
                    let b = sys::run("powercfg", &["/setactive", &s]);
                    format!("{a:?} {b:?}")
                })
            },
        ),
        (
            "API write lid.ac = 0 + SetActive".into(),
            Box::new(move || {
                let a = power::write(&copy, LID, Source::Ac, 0);
                let b = power::set_active(&copy);
                format!("{} {}", power::err_text(a), power::err_text(b))
            }),
        ),
        (
            format!("API write back sleep.ac {base}, lid.ac {lid_ac}, lid.dc {lid_dc} + SetActive"),
            Box::new(move || {
                let a = power::write(&copy, SLEEP, Source::Ac, base);
                let b = power::write(&copy, LID, Source::Ac, lid_ac);
                let c = power::write(&copy, LID, Source::Dc, lid_dc);
                let d = power::set_active(&copy);
                format!("{a} {b} {c} {d}")
            }),
        ),
    ];
    for (name, f) in steps {
        if ctrl::stop_requested() {
            log!("detect", "stop requested: skipping the rest");
            return;
        }
        let t0 = mark(&name);
        let out = f();
        let t1 = Instant::now();
        log!("detect", "{name}: {out}");
        let _ = ctrl::nap(2500);
        report(&name, t0, t1, rows);
    }
}

fn copy_test(keep: bool) -> i32 {
    let original = power::active_scheme().expect("active scheme");
    log!(
        "copy",
        "active scheme {} \"{}\"",
        guid_str(&original),
        power::friendly_name(&original)
    );
    for (k, v) in watch::snapshot(true) {
        log!("copy", "before: {k} = {v}");
    }
    log!(
        "copy",
        "PowerSettingAccessCheckEx(ACCESS_CREATE_SCHEME) write {}, (ACCESS_ACTIVE_SCHEME) write {}",
        power::err_text(power::access_check(ACCESS_CREATE_SCHEME, None, KEY_WRITE)),
        power::err_text(power::access_check(ACCESS_ACTIVE_SCHEME, None, KEY_WRITE))
    );
    let det = Detectors::start(Options {
        notify: vec![
            power::GUID_ACTIVE_POWERSCHEME,
            power::GUID_POWERSCHEME_PERSONALITY,
            power::LIDACTION,
            power::STANDBYIDLE,
        ],
        registry: true,
        poll_ms: Some(250),
        full: false,
        effective_mode: true,
        suspend_resume: false,
    });
    sleep(Duration::from_millis(1500));
    let mut rows = Vec::new();
    let t0 = mark("PowerDuplicateScheme(active)");
    let copy = match power::duplicate(&original) {
        Ok(g) => g,
        Err(rc) => {
            log!(
                "copy",
                "PowerDuplicateScheme failed: {}",
                power::err_text(rc)
            );
            det.stop();
            return 1;
        }
    };
    let t1 = Instant::now();
    let j = Journal {
        active_before: Some(original),
        copies: vec![copy],
        ..Default::default()
    };
    journal::save(&journal_path(), &j);
    log!("copy", "copy {}", guid_str(&copy));
    let rc = power::write_friendly_name(&copy, "Wakeshot spike copy");
    log!("copy", "PowerWriteFriendlyName rc {}", power::err_text(rc));
    let _ = ctrl::nap(2500);
    report("PowerDuplicateScheme", t0, t1, &mut rows);
    let t0 = mark("PowerSetActiveScheme(copy)");
    let rc = if ctrl::stop_requested() {
        log!("copy", "stop requested: the copy is not made active");
        1
    } else {
        power::set_active(&copy)
    };
    let t1 = Instant::now();
    log!(
        "copy",
        "PowerSetActiveScheme(copy) rc {}",
        power::err_text(rc)
    );
    let _ = ctrl::nap(3000);
    report("PowerSetActiveScheme(copy)", t0, t1, &mut rows);
    if !ctrl::stop_requested() {
        for (k, v) in watch::snapshot(true) {
            log!("copy", "copy active: {k} = {v}");
        }
    }
    if rc == 0 {
        detect_sequence(&copy, &mut rows);
    }
    let rc = power::delete(&copy);
    log!(
        "copy",
        "PowerDeleteScheme(copy) while it is active: {}",
        power::err_text(rc)
    );
    let still = power::schemes().contains(&copy);
    log!("copy", "copy still listed after that: {still}");
    if keep && still && !ctrl::stop_requested() {
        det.stop();
        log!(
            "copy",
            "--keep: the copy stays active. Run `restore` to remove it."
        );
        return 0;
    }
    let t0 = mark("PowerSetActiveScheme(original)");
    let rc = power::set_active(&original);
    let t1 = Instant::now();
    log!(
        "copy",
        "PowerSetActiveScheme(original) rc {}",
        power::err_text(rc)
    );
    let _ = ctrl::nap(3000);
    report("PowerSetActiveScheme(original)", t0, t1, &mut rows);
    let t0 = mark("PowerDeleteScheme(copy)");
    let rc = power::delete(&copy);
    let t1 = Instant::now();
    log!("copy", "PowerDeleteScheme(copy) rc {}", power::err_text(rc));
    let listed = power::schemes().contains(&copy);
    let back = power::active_scheme().ok() == Some(original);
    // rc 2 means already gone: a stop before the copy went active lets the earlier delete work.
    let clean = back && !listed && (rc == 0 || rc == 2);
    // Before the slow logging below, so a closed window still ends with the journal gone.
    if clean {
        let _ = std::fs::remove_file(journal_path());
        ctrl::mark_clean();
    }
    let _ = ctrl::nap(2500);
    report("PowerDeleteScheme(copy)", t0, t1, &mut rows);
    det.stop();
    let key = format!(r"{}\{}", sys::SCHEMES_KEY, guid_str(&copy));
    let key_left = sys::reg_string(sys::hklm(), &key, "FriendlyName");
    log!(
        "copy",
        "after delete: listed {listed}, registry FriendlyName {key_left:?}"
    );
    let (_, list) = sys::run("powercfg", &["/list"]);
    log!("copy", "powercfg /list: {}", list.replace('\n', " | "));
    for (k, v) in watch::snapshot(true) {
        log!("copy", "after: {k} = {v}");
    }
    log!(
        "result",
        "| trigger | call time | setting notification | registry notification | poll 250 ms |"
    );
    for r in &rows {
        log!("result", "{r}");
    }
    if clean {
        log!("copy", "clean: original active, copy gone, journal removed");
        0
    } else {
        log!("copy", "NOT CLEAN: run `restore`");
        1
    }
}

fn watch_mode(seconds: u64) -> i32 {
    let start = watch::snapshot(true);
    for (k, v) in &start {
        log!("watch", "start: {k} = {v}");
    }
    let det = Detectors::start(Options {
        notify: vec![
            power::LIDACTION,
            power::STANDBYIDLE,
            power::ESBATTTHRESHOLD,
            power::CONSOLELOCK,
            power::VIDEOIDLE,
            power::GUID_ACTIVE_POWERSCHEME,
            power::GUID_ACDC_POWER_SOURCE,
            power::GUID_ENERGY_SAVER_STATUS,
            power::GUID_CONSOLE_DISPLAY_STATE,
        ],
        registry: true,
        poll_ms: Some(500),
        full: true,
        effective_mode: true,
        suspend_resume: true,
    });
    log!(
        "watch",
        "watching for {seconds} s. Press Ctrl+C once to end early; the check below still runs."
    );
    if !ctrl::nap(seconds * 1000) {
        log!("watch", "ended early by Ctrl+C");
    }
    det.stop();
    let end = watch::snapshot(true);
    let changed = watch::diff(&start, &end);
    if changed.is_empty() {
        log!("watch", "every value is back to its start value");
    } else {
        for c in changed {
            log!("watch", "STILL CHANGED: {c}");
        }
    }
    0
}

enum Hold {
    None,
    Request(windows::Win32::Foundation::HANDLE),
    Stes,
}

fn hold(args: &Args) -> i32 {
    let sleep_dc = args.num("--sleep-dc", 60) as u32;
    let minutes = args.num("--minutes", 12);
    let kind = args
        .opt("--request")
        .unwrap_or_else(|| "system+execution".into());
    let ps = sys::power_status();
    if ps.on_ac && !args.flag("--allow-ac") {
        log!("hold", "refusing: on AC. Unplug the charger first.");
        return 3;
    }
    if let Ok(Some(_)) = journal::load(&journal_path()) {
        log!(
            "hold",
            "refusing: out/journal.txt exists. Run `restore` first."
        );
        return 4;
    }
    let original = power::active_scheme().expect("active scheme");
    let user_dc = power::read(&original, SLEEP, Source::Dc).unwrap_or(0);
    let copy = match power::duplicate(&original) {
        Ok(g) => g,
        Err(rc) => {
            log!(
                "hold",
                "PowerDuplicateScheme failed: {}",
                power::err_text(rc)
            );
            return 1;
        }
    };
    journal::save(
        &journal_path(),
        &Journal {
            active_before: Some(original),
            copies: vec![copy],
            ..Default::default()
        },
    );
    let _ = power::write_friendly_name(&copy, "Wakeshot spike battery test");
    let rc_w = power::write(&copy, SLEEP, Source::Dc, sleep_dc);
    let rc_a = power::set_active(&copy);
    log!(
        "hold",
        "copy {} active (rc {}), sleep.dc {sleep_dc} s (rc {}); user's own sleep.dc is {user_dc} s",
        guid_str(&copy),
        power::err_text(rc_a),
        power::err_text(rc_w)
    );
    for (k, v) in watch::snapshot(true) {
        log!("hold", "{k} = {v}");
    }
    let det = Detectors::start(Options {
        notify: vec![
            power::GUID_CONSOLE_DISPLAY_STATE,
            power::GUID_ACDC_POWER_SOURCE,
            power::GUID_ACTIVE_POWERSCHEME,
            power::GUID_ENERGY_SAVER_STATUS,
        ],
        registry: false,
        poll_ms: None,
        full: false,
        effective_mode: false,
        suspend_resume: true,
    });
    let mut reason: Vec<u16> = "Wakeshot power spike: battery rule test"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let held = match kind.as_str() {
        "none" => Hold::None,
        "stes" => {
            let prev = unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) };
            log!(
                "hold",
                "SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) previous {:#x}",
                prev.0
            );
            Hold::Stes
        }
        _ => {
            let ctx = REASON_CONTEXT {
                Version: POWER_REQUEST_CONTEXT_VERSION,
                Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
                Reason: REASON_CONTEXT_0 {
                    SimpleReasonString: PWSTR(reason.as_mut_ptr()),
                },
            };
            let h = unsafe { PowerCreateRequest(&ctx) }.expect("PowerCreateRequest");
            let r1 = unsafe { PowerSetRequest(h, PowerRequestSystemRequired) };
            log!("hold", "PowerSetRequest(SystemRequired) {r1:?}");
            if kind == "system+execution" {
                let r2 = unsafe { PowerSetRequest(h, PowerRequestExecutionRequired) };
                log!("hold", "PowerSetRequest(ExecutionRequired) {r2:?}");
            }
            Hold::Request(h)
        }
    };
    log!(
        "hold",
        "holding '{kind}' for {minutes} min. No request: sleep about {sleep_dc} s after the last input. \
         With the documented rule: requests end about {} s after the last input.",
        sleep_dc + 300
    );
    let wall0 = SystemTime::now();
    let unb0 = sys::unbiased_ms();
    let mut last_wall = wall0;
    let mut last_unb = unb0;
    let mut gaps = Vec::new();
    loop {
        if !ctrl::nap(5000) {
            log!("hold", "stop requested: restoring now");
            break;
        }
        let wall = SystemTime::now();
        let unb = sys::unbiased_ms();
        let wall_step = wall
            .duration_since(last_wall)
            .unwrap_or_default()
            .as_millis() as i64;
        let unb_step = (unb - last_unb) as i64;
        let ps = sys::power_status();
        let elapsed = wall.duration_since(wall0).unwrap_or_default().as_secs();
        log!(
            "tick",
            "t {elapsed} s, idle {} s, wall step {wall_step} ms, unbiased step {unb_step} ms, AC {}, battery {}%",
            sys::idle_ms() / 1000,
            ps.on_ac,
            ps.battery_percent
        );
        if wall_step > 15_000 || wall_step - unb_step > 3_000 {
            let g = format!(
                "GAP at t {elapsed} s ({}): wall step {wall_step} ms, unbiased step {unb_step} ms",
                log::wall_clock()
            );
            log!("tick", "{g}");
            gaps.push(g);
        }
        if ps.on_ac {
            log!(
                "tick",
                "WARNING: on AC, the battery rule does not apply now"
            );
        }
        last_wall = wall;
        last_unb = unb;
        if elapsed >= minutes * 60 {
            break;
        }
    }
    match held {
        Hold::None => {}
        Hold::Stes => unsafe {
            SetThreadExecutionState(ES_CONTINUOUS);
        },
        Hold::Request(h) => unsafe {
            let _ = PowerClearRequest(h, PowerRequestSystemRequired);
            if kind == "system+execution" {
                let _ = PowerClearRequest(h, PowerRequestExecutionRequired);
            }
            let _ = windows::Win32::Foundation::CloseHandle(h);
        },
    }
    let rc1 = power::set_active(&original);
    let rc2 = power::delete(&copy);
    log!(
        "hold",
        "restore: PowerSetActiveScheme(original) {}, PowerDeleteScheme(copy) {}",
        power::err_text(rc1),
        power::err_text(rc2)
    );
    let clean = power::active_scheme().ok() == Some(original) && !power::schemes().contains(&copy);
    // Before the slow event dump below, so a closed window still ends with the journal gone.
    if clean {
        let _ = std::fs::remove_file(journal_path());
        ctrl::mark_clean();
    }
    det.stop();
    // Only events since the test began; wevtutil measures timediff from now in ms.
    let since_ms = SystemTime::now()
        .duration_since(wall0)
        .unwrap_or_default()
        .as_millis()
        + 60_000;
    let query = format!(
        "/q:*[System[Provider[@Name='Microsoft-Windows-Kernel-Power'] and          (EventID=506 or EventID=507 or EventID=105 or EventID=42 or EventID=107) and          TimeCreated[timediff(@SystemTime) <= {since_ms}]]]"
    );
    let (_, ev) = sys::run(
        "wevtutil",
        &["qe", "System", &query, "/rd:false", "/f:text"],
    );
    for l in ev.lines() {
        let t = l.trim();
        if t.starts_with("Date:")
            || t.starts_with("Event ID:")
            || t.starts_with("Reason")
            || t.starts_with("The system")
        {
            log!("events", "{t}");
        }
    }
    log!(
        "result",
        "gaps: {}",
        if gaps.is_empty() {
            "none".to_string()
        } else {
            gaps.join(" / ")
        }
    );
    powercfg_lid_and_sleep();
    if clean {
        log!("hold", "clean: original plan active, copy removed");
        0
    } else {
        log!("hold", "NOT CLEAN: run `restore`");
        1
    }
}
