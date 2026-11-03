//! `wakeshot automatic on|off`: write `config.json`.
//!
//! Owner: sources track (`p4-cli-commands`); fixes by `p6-int-dry-run-e2e`.

use std::process::ExitCode;

use wakeshot_core::files::{self, Config, Read};
use wakeshot_windows::log::{Level, Log};
use wakeshot_windows::store::{self, DataDir, DataFile};

use crate::command::Global;
use crate::status::{self, AREA, Answer};

const NEWER: &str =
    "Your settings were saved by a newer Wakeshot, so this one leaves them. Update Wakeshot.";

pub fn main(global: &Global, on: bool) -> ExitCode {
    match status::open(global) {
        Ok((dir, log)) => run(&dir, &*log, on).print(),
        Err(answer) => answer.print(),
    }
}

/// Whether the tray runs or not: a running tray reads the file when it changes, a stopped one at its next start.
pub(crate) fn run(dir: &DataDir, log: &dyn Log, on: bool) -> Answer {
    let path = dir.file(DataFile::Config);
    let word = match on {
        true => "on",
        false => "off",
    };
    // A newer file may hold fields this version does not know; writing over it would drop them. A file that does not
    // parse is replaced: the user's choice is clear, and the tray ignores such a file anyway.
    match store::read_whole(&path) {
        Ok(Some(bytes)) => {
            if let Read::Newer(version) = files::read::<Config>(&bytes) {
                let text = format!(
                    "automatic {word}: not written, config.json is version {version}, newer than {}",
                    Config::VERSION
                );
                log.line(Level::Warn, AREA, &text);
                return Answer::failed(NEWER);
            }
        }
        Ok(None) => {}
        Err(e) => {
            let text = format!("automatic {word}: not written, {}", status::error_text(e));
            log.line(Level::Warn, AREA, &text);
            return Answer::failed(format!(
                "Could not read your settings: {}.",
                status::error_text(e)
            ));
        }
    }
    if let Err(e) = store::write_atomic(&path, config_json(on).as_bytes()) {
        let text = format!("automatic {word}: not written, {}", status::error_text(e));
        log.line(Level::Warn, AREA, &text);
        return Answer::failed(format!(
            "Could not save Automatic: {}.",
            status::error_text(e)
        ));
    }
    log.line(Level::Info, AREA, &format!("automatic {word}: written"));
    Answer::ok(format!("Automatic is {word}."))
}

/// `Config::new(on)` in the form serde writes it (architecture section 6). By hand: this crate has no JSON writer,
/// and the file holds a number and a bool only.
fn config_json(on: bool) -> String {
    let config = Config::new(on);
    format!(
        r#"{{"version":{},"automatic":{}}}"#,
        config.version, config.automatic
    )
}

#[cfg(test)]
mod tests {
    use wakeshot_windows::log::MemoryLog;

    use super::*;
    use crate::status::fixture::TempData;

    fn automatic(data: &TempData, on: bool) -> (Answer, Vec<String>) {
        let log = MemoryLog::default();
        let answer = run(&data.dir, &log, on);
        let lines = log.lines.into_inner().unwrap();
        (answer, lines.into_iter().map(|(_, _, text)| text).collect())
    }

    fn config(data: &TempData) -> Read<Config> {
        files::read(&data.read(DataFile::Config).expect("config.json"))
    }

    #[test]
    fn on_and_off_write_the_config() {
        let data = TempData::new("auto");
        let (answer, log) = automatic(&data, false);
        assert_eq!(answer, Answer::ok("Automatic is off."));
        assert_eq!(log, ["automatic off: written"]);
        assert_eq!(config(&data), Read::Value(Config::new(false)));

        let (answer, log) = automatic(&data, true);
        assert_eq!(answer, Answer::ok("Automatic is on."));
        assert_eq!(log, ["automatic on: written"]);
        assert_eq!(config(&data), Read::Value(Config::new(true)));
    }

    #[test]
    fn a_newer_config_is_kept() {
        let data = TempData::new("auto-newer");
        let newer = r#"{ "version": 2, "automatic": true, "theme": "dark" }"#;
        data.write(DataFile::Config, newer);
        let (answer, log) = automatic(&data, false);
        assert_eq!(answer, Answer::failed(NEWER));
        assert_eq!(
            log,
            ["automatic off: not written, config.json is version 2, newer than 1"]
        );
        assert_eq!(data.read(DataFile::Config).unwrap(), newer.as_bytes());
    }

    #[test]
    fn a_bad_or_older_config_is_replaced() {
        for old in ["{ not json", r#"{ "version": 0, "automatic": true }"#] {
            let data = TempData::new("auto-bad");
            data.write(DataFile::Config, old);
            assert_eq!(automatic(&data, false).0, Answer::ok("Automatic is off."));
            assert_eq!(config(&data), Read::Value(Config::new(false)));
        }
    }
}
