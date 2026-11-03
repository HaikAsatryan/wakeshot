//! `wakeshot log`: open the log with the default app.
//!
//! Owner: sources track (`p4-cli-commands`); fixes by `p6-int-dry-run-e2e`.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::ExitCode;

use wakeshot_windows::WinError;
use wakeshot_windows::log::{Level, Proc};
use wakeshot_windows::recovery;
use wakeshot_windows::shell;
use wakeshot_windows::store::DataDir;

use crate::command::Global;
use crate::status::{self, AREA, Answer};

/// `log::LOG_FILE`, private there: the one file every process appends to.
const LOG_FILE: &str = "wakeshot.log";

const NO_LOG: &str = "No log yet. Wakeshot writes one once it runs.";

pub fn main(global: &Global) -> ExitCode {
    match status::data_dir(global) {
        Ok(dir) => run(&dir, &shell::open_with_default_app).print(),
        Err(answer) => answer.print(),
    }
}

pub(crate) fn log_path(dir: &DataDir) -> PathBuf {
    dir.logs().join(LOG_FILE)
}

/// `open`: the shell in `main`, a fake in tests, so no test opens a window.
pub(crate) fn run(dir: &DataDir, open: &dyn Fn(&OsStr) -> Result<(), WinError>) -> Answer {
    let path = log_path(dir);
    // No log line here: it would create the very file this says is missing. A folder that cannot be read gets the
    // open, which then names the reason.
    if matches!(path.try_exists(), Ok(false)) {
        return Answer::ok(NO_LOG);
    }
    let log = recovery::open_log(dir, Proc::Cli);
    match open(path.as_os_str()) {
        Ok(()) => {
            log.line(Level::Info, AREA, "log: opened");
            Answer::ok(format!("Opened the log: {}", path.display()))
        }
        Err(e) => {
            let text = format!("log: not opened, {}", status::error_text(e));
            log.line(Level::Warn, AREA, &text);
            Answer::failed(format!(
                "Could not open the log ({}). It is {}",
                status::error_text(e),
                path.display()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::status::fixture::TempData;

    #[test]
    fn no_log_yet_opens_nothing_and_writes_nothing() {
        let data = TempData::new("log-none");
        let asked = RefCell::new(Vec::new());
        let answer = run(&data.dir, &|p| {
            asked.borrow_mut().push(p.to_owned());
            Ok(())
        });
        assert_eq!(answer, Answer::ok(NO_LOG));
        assert!(asked.borrow().is_empty());
        assert!(!data.dir.logs().exists());
    }

    #[test]
    fn opens_the_log_file_of_the_data_folder() {
        let data = TempData::new("log-open");
        let path = data.dir.logs().join("wakeshot.log");
        std::fs::create_dir_all(data.dir.logs()).unwrap();
        std::fs::write(&path, "").unwrap();
        let asked = RefCell::new(Vec::new());
        let answer = run(&data.dir, &|p| {
            asked.borrow_mut().push(p.to_owned());
            Ok(())
        });
        assert_eq!(
            answer,
            Answer::ok(format!("Opened the log: {}", path.display()))
        );
        assert_eq!(*asked.borrow(), [path.as_os_str().to_owned()]);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.trim_end().ends_with("cli: log: opened"), "{text}");
    }

    #[test]
    fn a_failed_open_names_the_file() {
        let data = TempData::new("log-fail");
        let path = data.dir.logs().join("wakeshot.log");
        std::fs::create_dir_all(data.dir.logs()).unwrap();
        std::fs::write(&path, "").unwrap();
        let answer = run(&data.dir, &|_| {
            Err(WinError {
                call: "ShellExecuteExW",
                code: 1155,
            })
        });
        assert_eq!(
            answer,
            Answer::failed(format!(
                "Could not open the log (ShellExecuteExW failed with 1155). It is {}",
                path.display()
            ))
        );
    }

    /// `wakeshot log` for real, with a person at the screen (`--ignored`): a temp log opens in the default app for
    /// `.log` files. Not for an unattended run: Windows 11 Notepad opens a file as a new tab in a window the user may
    /// have open with unsaved work.
    #[test]
    #[ignore = "opens a window: run by hand with a person at the screen"]
    fn real_open_log() {
        let data = TempData::new("log-real");
        std::fs::create_dir_all(data.dir.logs()).unwrap();
        std::fs::write(
            log_path(&data.dir),
            "Wakeshot test log: close this window.\r\n",
        )
        .unwrap();
        let answer = run(&data.dir, &shell::open_with_default_app);
        assert_eq!(answer.code, 0, "{answer:?}");
        println!("{}", answer.text);
        // The app reads the file before the folder goes.
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}
