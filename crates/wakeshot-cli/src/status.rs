//! `wakeshot status`: what the tray is doing, from `state.json`. Also the parts every command of this crate shares:
//! the data folder `--dry-run` picks, the log, the tray's state and how an answer is printed.
//!
//! Owner: sources track (`p4-cli-commands`); fixes by `p6-int-dry-run-e2e`.

use std::collections::BTreeSet;
use std::io::Write;
use std::process::ExitCode;
use std::sync::Arc;

use wakeshot_core::files::{self, Read, StateFile};
use wakeshot_core::process::{Alive, ProcessRef};
use wakeshot_core::settings::{OwnedKey, SettingKey};
use wakeshot_windows::WinError;
use wakeshot_windows::log::{Level, Log, Proc};
use wakeshot_windows::process::{Liveness, SystemLiveness};
use wakeshot_windows::recovery;
use wakeshot_windows::store::{self, DataDir, DataFile};

use crate::command::Global;

/// The log area of every CLI line.
pub(crate) const AREA: &str = "cli";

pub(crate) const UNKNOWN: &str = "Cannot tell whether Wakeshot is running. Try again.";
pub(crate) const NEWER: &str = "The running Wakeshot is newer than this command. Update Wakeshot.";
pub(crate) const UNREADABLE: &str = "Cannot read what Wakeshot is doing. Try again.";

pub fn main(global: &Global) -> ExitCode {
    match open(global) {
        Ok((dir, log)) => run(&dir, &SystemLiveness, &*log).print(),
        Err(answer) => answer.print(),
    }
}

pub(crate) fn run(dir: &DataDir, liveness: &dyn Liveness, log: &dyn Log) -> Answer {
    let tray = tray(dir, liveness);
    log.line(tray.level(), AREA, &format!("status: {}", tray.describe()));
    match tray {
        Tray::Running(state) => Answer::ok(report(&state)),
        Tray::NotRunning => Answer::ok("Wakeshot is not running."),
        Tray::Unknown => Answer::failed(UNKNOWN),
        Tray::Newer(_) => Answer::failed(NEWER),
        Tray::Unreadable(_) => Answer::failed(UNREADABLE),
    }
}

/// One fact per line, so a script can pick the line it needs.
fn report(state: &StateFile) -> String {
    let mut lines = vec!["Wakeshot is running.".to_owned()];
    lines.push(match state.automatic {
        true => "Automatic is on.".to_owned(),
        false => "Automatic is off.".to_owned(),
    });
    if state.stepped_back {
        lines.push("Battery low, so Wakeshot is not holding the PC awake.".to_owned());
        lines.extend(
            state
                .holding
                .iter()
                .map(|h| format!("Wants the PC awake: {h}")),
        );
    } else if state.holding.is_empty() {
        lines.push("Nothing holds the PC awake.".to_owned());
    } else {
        lines.extend(state.holding.iter().map(|h| format!("Holding awake: {h}")));
    }
    if state.overridden.is_empty() {
        lines.push("Wakeshot has not changed your power settings.".to_owned());
    } else {
        lines.extend(overridden(&state.overridden));
    }
    lines.join("\n")
}

/// The plan shows only when keys of more than one plan are ours: a plan switch during a hold keeps the old plan's
/// keys apart, and then the user needs to know which plan each one is in.
pub(crate) fn overridden(keys: &[OwnedKey]) -> Vec<String> {
    let keys: BTreeSet<OwnedKey> = keys.iter().copied().collect();
    let plans: BTreeSet<_> = keys.iter().map(|k| k.plan).collect();
    keys.iter()
        .map(|k| match plans.len() {
            1 => format!("Changed by Wakeshot: {}", setting_words(k.key)),
            _ => format!(
                "Changed by Wakeshot in power plan {}: {}",
                k.plan,
                setting_words(k.key)
            ),
        })
        .collect()
}

/// The words of the tray's notices (`text::setting`, private there), so the CLI and the tray name a setting alike;
/// `setting_words_match_the_notices` keeps them equal.
pub(crate) fn setting_words(key: SettingKey) -> &'static str {
    match key {
        SettingKey::LidAc => "what closing the lid does when plugged in",
        SettingKey::LidDc => "what closing the lid does on battery",
        SettingKey::SleepDc => "when the PC sleeps on battery",
    }
}

/// What a command prints and its exit code: 0 goes to stdout, anything else to stderr.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Answer {
    pub code: u8,
    pub text: String,
}

impl Answer {
    pub(crate) fn ok(text: impl Into<String>) -> Self {
        Self {
            code: 0,
            text: text.into(),
        }
    }

    pub(crate) fn failed(text: impl Into<String>) -> Self {
        Self {
            code: 1,
            text: text.into(),
        }
    }

    pub(crate) fn print(self) -> ExitCode {
        // Write errors are ignored: `println!` would panic on a closed pipe, and the release build aborts on panic.
        let _ = match self.code {
            0 => writeln!(std::io::stdout(), "{}", self.text),
            _ => writeln!(std::io::stderr(), "{}", self.text),
        };
        ExitCode::from(self.code)
    }
}

/// `%LocalAppData%\Wakeshot`, or its `dry-run` folder, as `wakeshot-tray --dry-run` uses it (track note 13).
pub(crate) fn data_dir(global: &Global) -> Result<DataDir, Answer> {
    let dir = match global.dry_run {
        true => DataDir::dry_run(),
        false => DataDir::real(),
    };
    dir.map_err(|e| {
        Answer::failed(format!(
            "Cannot find Wakeshot's data folder: {}.",
            error_text(e)
        ))
    })
}

/// The data folder and the log every command starts with. A log that does not open logs nothing; the command still
/// runs.
pub(crate) fn open(global: &Global) -> Result<(DataDir, Arc<dyn Log>), Answer> {
    let dir = data_dir(global)?;
    let log = recovery::open_log(&dir, Proc::Cli);
    Ok((dir, log))
}

pub(crate) fn error_text(e: WinError) -> String {
    format!("{} failed with {}", e.call, e.code)
}

/// What `state.json` says about the tray.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Tray {
    /// The tray that wrote the file runs.
    Running(StateFile),
    /// No file, or the process that wrote it is gone.
    NotRunning,
    /// The liveness check itself failed: nothing may be decided on it (architecture section 5, "Liveness").
    Unknown,
    Newer(u32),
    /// The file does not read or does not parse; the text is for the log, never file content.
    Unreadable(String),
}

impl Tray {
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Running(state) => format!("running (pid {})", state.pid),
            Self::NotRunning => "not running".to_owned(),
            Self::Unknown => "cannot tell whether the tray runs".to_owned(),
            Self::Newer(version) => format!(
                "state.json is version {version}, newer than {}",
                StateFile::VERSION
            ),
            Self::Unreadable(why) => format!("state.json unreadable: {why}"),
        }
    }

    pub(crate) fn level(&self) -> Level {
        match self {
            Self::Running(_) | Self::NotRunning => Level::Info,
            Self::Unknown | Self::Newer(_) | Self::Unreadable(_) => Level::Warn,
        }
    }
}

/// The tray's pid and start in `state.json`, checked: the file stays after the tray ends, so only liveness says it
/// still runs.
pub(crate) fn tray(dir: &DataDir, liveness: &dyn Liveness) -> Tray {
    let bytes = match store::read_whole(&dir.file(DataFile::State)) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Tray::NotRunning,
        Err(e) => return Tray::Unreadable(error_text(e)),
    };
    let state: StateFile = match files::read(&bytes) {
        Read::Value(state) => state,
        Read::Newer(version) => return Tray::Newer(version),
        Read::Bad => return Tray::Unreadable("it does not parse".to_owned()),
    };
    let process = ProcessRef {
        pid: state.pid,
        start: state.proc_start,
    };
    match liveness.check(process) {
        Alive::Yes => Tray::Running(state),
        Alive::Gone => Tray::NotRunning,
        Alive::Unknown => Tray::Unknown,
    }
}

/// Shared by the tests of every command here: a temp data folder and the files the tray would write.
#[cfg(test)]
pub(crate) mod fixture {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use wakeshot_core::process::ProcessRef;
    use wakeshot_windows::store::{DataDir, DataFile};

    /// A fresh folder under the temp folder, removed on drop.
    pub(crate) struct TempData {
        pub dir: DataDir,
    }

    static NEXT: AtomicU32 = AtomicU32::new(0);

    impl TempData {
        pub(crate) fn new(tag: &str) -> Self {
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let root: PathBuf =
                std::env::temp_dir().join(format!("wakeshot-cli-{tag}-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            Self {
                dir: DataDir::at(root),
            }
        }

        pub(crate) fn path(&self, file: DataFile) -> PathBuf {
            self.dir.file(file)
        }

        pub(crate) fn write(&self, file: DataFile, text: &str) {
            let path = self.path(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }

        pub(crate) fn read(&self, file: DataFile) -> Option<Vec<u8>> {
            std::fs::read(self.path(file)).ok()
        }
    }

    impl Drop for TempData {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.dir.root());
        }
    }

    /// This test process: alive for as long as the test runs.
    pub(crate) fn me() -> ProcessRef {
        wakeshot_windows::process::current().unwrap()
    }

    /// A process that ran and ended: its pid with its own start time names no live process again.
    pub(crate) fn dead() -> ProcessRef {
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "exit"])
            .spawn()
            .unwrap();
        let start = wakeshot_windows::process::start_of(&child).unwrap();
        child.wait().unwrap();
        ProcessRef {
            pid: child.id(),
            start,
        }
    }

    /// `state.json` as the tray writes it; `overridden` as (plan, key name) pairs.
    pub(crate) fn state_json(
        tray: ProcessRef,
        automatic: bool,
        holding: &[&str],
        overridden: &[(&str, &str)],
        stepped_back: bool,
    ) -> String {
        let holding: Vec<String> = holding.iter().map(|h| format!("\"{h}\"")).collect();
        let overridden: Vec<String> = overridden
            .iter()
            .map(|(plan, key)| format!(r#"{{"plan":"{plan}","key":"{key}"}}"#))
            .collect();
        format!(
            r#"{{"version":1,"pid":{},"procStart":"{}","automatic":{automatic},"holding":[{}],"overridden":[{}],"steppedBack":{stepped_back}}}"#,
            tray.pid,
            tray.start.0,
            holding.join(","),
            overridden.join(",")
        )
    }
}

#[cfg(test)]
mod tests {
    use wakeshot_core::text;
    use wakeshot_core::view::Notice;
    use wakeshot_windows::log::MemoryLog;
    use wakeshot_windows::process::FakeLiveness;

    use super::fixture::{TempData, dead, me, state_json};
    use super::*;

    const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
    const SAVER: &str = "a1841308-3334-4063-b338-1d6f37f6b0f6";

    fn status(data: &TempData) -> (Answer, Vec<String>) {
        let log = MemoryLog::default();
        let answer = run(&data.dir, &SystemLiveness, &log);
        let lines = log.lines.into_inner().unwrap();
        (answer, lines.into_iter().map(|(_, _, text)| text).collect())
    }

    #[test]
    fn not_running_without_a_state_file() {
        let data = TempData::new("status-none");
        let (answer, log) = status(&data);
        assert_eq!(answer, Answer::ok("Wakeshot is not running."));
        assert_eq!(log, ["status: not running"]);
    }

    #[test]
    fn not_running_when_the_tray_in_the_file_is_gone() {
        let data = TempData::new("status-dead");
        data.write(DataFile::State, &state_json(dead(), true, &[], &[], false));
        assert_eq!(status(&data).0, Answer::ok("Wakeshot is not running."));
    }

    /// A reused pid is not the tray: the start time must match too.
    #[test]
    fn not_running_when_the_pid_runs_with_another_start() {
        let data = TempData::new("status-reused");
        let mut other = me();
        other.start.0 += 1;
        data.write(DataFile::State, &state_json(other, true, &[], &[], false));
        assert_eq!(status(&data).0, Answer::ok("Wakeshot is not running."));
    }

    #[test]
    fn running_with_holds_and_overrides() {
        let data = TempData::new("status-holds");
        let state = state_json(
            me(),
            true,
            &["Claude Code (2 sessions)", "Keep awake (25 min left)"],
            &[(BALANCED, "lidDc"), (BALANCED, "sleepDc")],
            false,
        );
        data.write(DataFile::State, &state);
        let (answer, log) = status(&data);
        assert_eq!(
            answer,
            Answer::ok(
                "Wakeshot is running.\n\
                 Automatic is on.\n\
                 Holding awake: Claude Code (2 sessions)\n\
                 Holding awake: Keep awake (25 min left)\n\
                 Changed by Wakeshot: what closing the lid does on battery\n\
                 Changed by Wakeshot: when the PC sleeps on battery"
            )
        );
        assert_eq!(log, [format!("status: running (pid {})", me().pid)]);
    }

    #[test]
    fn running_idle_with_automatic_off() {
        let data = TempData::new("status-idle");
        data.write(DataFile::State, &state_json(me(), false, &[], &[], false));
        assert_eq!(
            status(&data).0,
            Answer::ok(
                "Wakeshot is running.\n\
                 Automatic is off.\n\
                 Nothing holds the PC awake.\n\
                 Wakeshot has not changed your power settings."
            )
        );
    }

    #[test]
    fn stepped_back_names_what_wants_the_pc_awake() {
        let data = TempData::new("status-floor");
        let state = state_json(me(), true, &["nightly backup"], &[], true);
        data.write(DataFile::State, &state);
        assert_eq!(
            status(&data).0,
            Answer::ok(
                "Wakeshot is running.\n\
                 Automatic is on.\n\
                 Battery low, so Wakeshot is not holding the PC awake.\n\
                 Wants the PC awake: nightly backup\n\
                 Wakeshot has not changed your power settings."
            )
        );
    }

    #[test]
    fn overrides_in_two_plans_name_the_plan() {
        let data = TempData::new("status-plans");
        let state = state_json(
            me(),
            true,
            &[],
            &[(SAVER, "lidAc"), (BALANCED, "lidDc")],
            false,
        );
        data.write(DataFile::State, &state);
        let text = status(&data).0.text;
        let changed: Vec<&str> = text.lines().filter(|l| l.starts_with("Changed")).collect();
        assert_eq!(
            changed,
            [
                format!(
                    "Changed by Wakeshot in power plan {BALANCED}: what closing the lid does on battery"
                ),
                format!(
                    "Changed by Wakeshot in power plan {SAVER}: what closing the lid does when plugged in"
                ),
            ]
        );
    }

    #[test]
    fn newer_state_file_asks_for_an_update() {
        let data = TempData::new("status-newer");
        data.write(
            DataFile::State,
            r#"{ "version": 2, "pid": "changed shape", "holding": {} }"#,
        );
        let (answer, log) = status(&data);
        assert_eq!(answer, Answer::failed(NEWER));
        assert_eq!(log, ["status: state.json is version 2, newer than 1"]);
    }

    #[test]
    fn bad_state_file_is_unreadable() {
        let data = TempData::new("status-bad");
        data.write(DataFile::State, "{ not json");
        let (answer, log) = status(&data);
        assert_eq!(answer, Answer::failed(UNREADABLE));
        assert_eq!(log, ["status: state.json unreadable: it does not parse"]);
    }

    #[test]
    fn unknown_liveness_decides_nothing() {
        let data = TempData::new("status-unknown");
        let tray = me();
        data.write(DataFile::State, &state_json(tray, true, &[], &[], false));
        let liveness = FakeLiveness::default();
        liveness.alive.lock().unwrap().push((tray, Alive::Unknown));
        let answer = run(&data.dir, &liveness, &MemoryLog::default());
        assert_eq!(answer, Answer::failed(UNKNOWN));
    }

    /// The tray and the CLI name a setting in the same words.
    #[test]
    fn setting_words_match_the_notices() {
        for key in [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc] {
            let body = text::notice(&Notice::UserChanged(key)).body;
            let words = format!("You changed {}.", setting_words(key));
            assert!(body.contains(&words), "{body:?} lacks {words:?}");
        }
    }

    #[test]
    fn fixture_state_reads_as_the_tray_writes_it() {
        let tray = me();
        let text = state_json(tray, true, &["a"], &[(BALANCED, "lidAc")], true);
        let Read::Value(state) = files::read::<StateFile>(text.as_bytes()) else {
            panic!("{text}");
        };
        assert_eq!((state.pid, state.proc_start), (tray.pid, tray.start));
        assert_eq!(state.overridden[0].key, SettingKey::LidAc);
        assert!(state.stepped_back);
    }

    /// The card's `--dry-run` checks through each command's `main`, as the exe runs them once `args::parse` routes
    /// them (`p4-cli-args`). Uses the real dry-run folder with no dry-run tray running: run by hand, never beside
    /// `p6-install-e2e`. `config.json` is put back as it was.
    #[test]
    #[ignore = "uses %LocalAppData%\\Wakeshot\\dry-run: run by hand"]
    fn dry_run_folder_checks() {
        use wakeshot_core::files::Config;
        use wakeshot_core::view::KeepAwake;

        let dry = Global { dry_run: true };
        let dir = data_dir(&dry).unwrap();
        assert!(dir.root().ends_with(r"Wakeshot\dry-run"), "{dir:?}");
        let config = dir.file(DataFile::Config);
        let before = std::fs::read(&config).ok();

        assert_eq!(main(&dry), ExitCode::SUCCESS);
        assert_eq!(
            crate::keep_awake::main(&dry, KeepAwake::Minutes(60), false),
            ExitCode::from(1)
        );
        assert!(!dir.file(DataFile::KeepAwake).exists());
        assert_eq!(crate::automatic::main(&dry, false), ExitCode::SUCCESS);
        let written = std::fs::read(&config).unwrap();
        println!("config.json: {}", String::from_utf8_lossy(&written));
        assert_eq!(files::read(&written), Read::Value(Config::new(false)));

        match before {
            Some(bytes) => std::fs::write(&config, bytes).unwrap(),
            None => std::fs::remove_file(&config).unwrap(),
        }
    }
}
