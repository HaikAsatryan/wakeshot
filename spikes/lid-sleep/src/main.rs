mod display;
mod events;
mod heat;
mod journal;
mod log;
mod modes;
mod plan;
mod power;
mod sound;
mod sys;
mod ticks;

use std::path::{Path, PathBuf};

const USAGE: &str = "usage: lid-sleep-spike <mode> [options]
modes (no physical action):
  read                        capabilities, hibernate, wake timer setting, lid, displays, heat, power info
  heat [--seconds N] [--load-threads T] [--load-at S] [--load-seconds L] [--heat-limit C]
                              log thermal zones every 2 s, with an optional CPU load that stops
                              early at C (default 85)
  timer-check [--after S]     arm a wake timer while awake and see what Windows says about it
  restore                     undo whatever out/journal.txt says the spike changed
  audio-read                  default playback devices, endpoint volumes, audio sessions (System Sounds)
  sound-test [--chime-only]   lid open: Phase 0 alias sound, then our chime on the picked device, with
                              loopback listeners on every playback endpoint
modes for Haik's morning steps:
  lid [--end release|screen-off|sleep|hibernate|none] [--work S] [--no-lock] [--no-sound]
      [--sound chime|alias] [--alias-after S] [--chime-after S] [--listen] [--heat-limit C]
                              plan copy with lid Do nothing, hold, react to lid close, then end work
                              (early if the hottest zone reaches C, default 85)
  suspend --method sleep|hibernate|screen-off [--wake-after S]
                              call one sleep method with the lid open
  wake [--after S] [--wake-timers 0|1|2]
                              wake timer test: you put the PC to sleep with Start > Power > Sleep
  resume [--retake] [--request power|stes] [--idle S]
                              is a held request still honored after a manual sleep and resume?
  limited <mode> [options]    run a mode with Explorer's unelevated token and wait for it
options: --log <file>, --allow-elevated, --max-minutes M, dry runs: --simulate-close-after S,
         --simulate-resume-after S, --dry";

pub fn out_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("out")
}

pub struct Args(pub Vec<String>);

impl Args {
    pub fn flag(&self, name: &str) -> bool {
        self.0.iter().any(|a| a == name)
    }
    pub fn opt(&self, name: &str) -> Option<String> {
        let i = self.0.iter().position(|a| a == name)?;
        self.0.get(i + 1).cloned()
    }
    pub fn num(&self, name: &str, default: u64) -> u64 {
        self.opt(name)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
    pub fn num_opt(&self, name: &str) -> Option<u64> {
        self.opt(name).and_then(|v| v.parse().ok())
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
    if mode == "limited" {
        std::process::exit(limited(&raw[1..]));
    }
    let args = Args(raw);
    let log_path = args
        .opt("--log")
        .map(PathBuf::from)
        .unwrap_or_else(|| out_dir().join(format!("{mode}-{}.log", stamp())));
    log::init(Some(&log_path));
    let tok = sys::token_info();
    let ps = sys::power_status();
    log!(
        "start",
        "mode {mode}, args {:?}, pid {}, elevated {}, integrity {} (0x{:x}), on AC {}, battery {}%, energy saver on {}, idle {} s",
        &args.0[1..],
        std::process::id(),
        tok.elevated,
        tok.integrity_name(),
        tok.integrity_rid,
        ps.on_ac,
        ps.battery_percent,
        ps.energy_saver_on,
        sys::idle_ms() / 1000
    );
    let acts = matches!(mode.as_str(), "lid" | "suspend" | "wake" | "resume");
    if acts && tok.elevated && !args.flag("--allow-elevated") {
        log!(
            "start",
            "refusing: elevated token. Wakeshot runs as a normal user. Open C:\\Users\\haika\\code\\wakeshot\\spikes\\morning-shell.cmd from File Explorer and run it there."
        );
        std::process::exit(3);
    }
    if acts {
        events::install_ctrl_handler();
    }
    let code = match mode.as_str() {
        "read" => modes::read(),
        "heat" => modes::heat(&args),
        "timer-check" => modes::timer_check(&args),
        "lid" => modes::lid(&args),
        "suspend" => modes::suspend(&args),
        "wake" => modes::wake(&args),
        "resume" => modes::resume(&args),
        "restore" => plan::restore_mode(),
        "audio-read" => {
            sound::read_state("audio");
            0
        }
        "sound-test" => sound::sound_test(&args),
        _ => {
            println!("{USAGE}");
            2
        }
    };
    events::mark_clean();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_parse() {
        let a = Args(
            ["lid", "--work", "30", "--no-lock", "--end", "sleep"]
                .map(String::from)
                .to_vec(),
        );
        assert_eq!(a.num("--work", 60), 30);
        assert_eq!(a.num("--max-minutes", 12), 12);
        assert!(a.flag("--no-lock"));
        assert!(!a.flag("--no-sound"));
        assert_eq!(a.opt("--end").as_deref(), Some("sleep"));
        assert_eq!(a.num_opt("--wake-after"), None);
    }
}
