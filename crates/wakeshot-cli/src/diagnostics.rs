//! `wakeshot diagnostics [folder]`: one text file for a bug report, saved on the Desktop or in the folder given, then
//! shown in Explorer (architecture section 8, "Log").

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use wakeshot_windows::WinError;
use wakeshot_windows::clock;
use wakeshot_windows::diagnostics::{self, Redact};
use wakeshot_windows::log::{Level, Log};
use wakeshot_windows::process::{Liveness, SystemLiveness};
use wakeshot_windows::shell;
use wakeshot_windows::store::DataDir;

use crate::command::Global;
use crate::status::{self, AREA, Answer};

/// Where the bug form is, so the answer can say where the file goes.
const BUG_FORM: &str =
    "https://github.com/HaikAsatryan/wakeshot/issues/new?template=bug-report.yml";

pub fn main(global: &Global, folder: Option<&OsStr>) -> ExitCode {
    match status::open(global) {
        Ok((dir, log)) => {
            let folder = folder.map(PathBuf::from);
            run(
                &dir,
                folder.as_deref(),
                &SystemLiveness,
                &*log,
                &shell::show_in_folder,
            )
            .print()
        }
        Err(answer) => answer.print(),
    }
}

/// `show`: Explorer in `main`, a fake in tests, so no test opens a window.
pub(crate) fn run(
    dir: &DataDir,
    folder: Option<&Path>,
    liveness: &dyn Liveness,
    log: &dyn Log,
    show: &dyn Fn(&Path) -> Result<(), WinError>,
) -> Answer {
    let folder = match folder {
        Some(folder) => folder.to_path_buf(),
        None => match diagnostics::desktop() {
            Ok(desktop) => desktop,
            Err(e) => {
                return Answer::failed(format!(
                    "Cannot find your Desktop ({}). Give a folder, such as: wakeshot diagnostics C:\\Temp",
                    status::error_text(e)
                ));
            }
        },
    };
    if !folder.is_dir() {
        return Answer::failed(format!(
            "There is no folder {}. Give a folder that exists.",
            folder.display()
        ));
    }
    // Before the logs are read, so the bundle shows when it was asked for.
    log.line(Level::Info, AREA, "diagnostics: making the bundle");
    let mut header: Vec<String> = diagnostics::machine_facts()
        .into_iter()
        .map(|fact| format!("{}: {}", fact.name, fact.value))
        .collect();
    header.push(format!("Tray: {}", status::tray(dir, liveness).describe()));
    if let Ok(exe) = std::env::current_exe() {
        header.push(format!("This command: {}", exe.display()));
    }
    let bundle = match diagnostics::write_bundle(
        dir,
        &folder,
        &header,
        &Redact::this_user(),
        clock::now().wall_ms,
    ) {
        Ok(bundle) => bundle,
        Err(e) => {
            let text = format!("diagnostics: not saved, {}", status::error_text(e));
            log.line(Level::Warn, AREA, &text);
            return Answer::failed(format!(
                "Could not save the file in {} ({}).",
                folder.display(),
                status::error_text(e)
            ));
        }
    };
    let text = format!(
        "diagnostics: saved, {} bytes, {} files",
        bundle.bytes, bundle.files
    );
    log.line(Level::Info, AREA, &text);
    let mut lines = vec![
        format!("Saved {}", bundle.path.display()),
        "It holds Wakeshot's own logs and settings files, with your user name and anything that looks like a token \
         taken out. Read it first if you like."
            .to_owned(),
        format!("Attach it to a bug report: {BUG_FORM}"),
    ];
    if let Err(e) = show(&bundle.path) {
        let text = format!(
            "diagnostics: Explorer not opened, {}",
            status::error_text(e)
        );
        log.line(Level::Warn, AREA, &text);
        lines.push(format!(
            "(Explorer did not open: {}.)",
            status::error_text(e)
        ));
    }
    Answer::ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use wakeshot_windows::log::{LogWriter, Proc};
    use wakeshot_windows::process::FakeLiveness;

    use super::*;
    use crate::status::fixture::TempData;

    #[test]
    fn saves_the_bundle_in_the_folder_and_shows_it() {
        let data = TempData::new("diag");
        LogWriter::open(&data.dir.logs(), Proc::Tray).unwrap().line(
            Level::Info,
            "core",
            "an earlier line",
        );
        let out = data.dir.root().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let shown = RefCell::new(Vec::new());
        let log = LogWriter::open(&data.dir.logs(), Proc::Cli).unwrap();
        let answer = run(
            &data.dir,
            Some(&out),
            &FakeLiveness::default(),
            &log,
            &|p| {
                shown.borrow_mut().push(p.to_path_buf());
                Ok(())
            },
        );
        assert_eq!(answer.code, 0, "{answer:?}");
        let shown = shown.borrow();
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].parent(), Some(out.as_path()));
        assert!(
            answer
                .text
                .starts_with(&format!("Saved {}", shown[0].display()))
        );
        assert!(answer.text.contains(BUG_FORM));
        let text = std::fs::read_to_string(&shown[0]).unwrap();
        assert!(text.contains("== logs\\wakeshot.log =="), "{text}");
        assert!(text.contains("core: an earlier line"));
        assert!(text.contains("cli: diagnostics: making the bundle"));
        assert!(text.contains("\r\nTray: not running\r\n"), "{text}");
        assert!(text.contains("\r\nWindows: Windows "), "{text}");
        let log_text = std::fs::read_to_string(data.dir.logs().join("wakeshot.log")).unwrap();
        assert!(log_text.contains("cli: diagnostics: saved, "), "{log_text}");
    }

    /// The bundle of this PC's real data folder, written to the folder `WAKESHOT_DIAG_OUT` names, with no Explorer
    /// window (`--ignored`); the caller then reads it for the user's name.
    #[test]
    #[ignore = "reads the real data folder: run by hand with WAKESHOT_DIAG_OUT set"]
    fn real_bundle_of_this_pc() {
        let out = PathBuf::from(std::env::var_os("WAKESHOT_DIAG_OUT").expect("WAKESHOT_DIAG_OUT"));
        let dir = DataDir::real().unwrap();
        let log = wakeshot_windows::recovery::open_log(&dir, Proc::Cli);
        let answer = run(&dir, Some(&out), &SystemLiveness, &*log, &|_| Ok(()));
        assert_eq!(answer.code, 0, "{answer:?}");
        println!("{}", answer.text);
    }

    #[test]
    fn a_missing_folder_saves_nothing() {
        let data = TempData::new("diag-missing");
        let missing = data.dir.root().join("no such folder");
        let answer = run(
            &data.dir,
            Some(&missing),
            &FakeLiveness::default(),
            &wakeshot_windows::testing::MemoryLog::default(),
            &|_| panic!("nothing to show"),
        );
        assert_eq!(
            answer,
            Answer::failed(format!(
                "There is no folder {}. Give a folder that exists.",
                missing.display()
            ))
        );
        assert!(!missing.exists());
    }

    #[test]
    fn explorer_that_does_not_open_still_saves() {
        let data = TempData::new("diag-noshow");
        let out = data.dir.root().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let answer = run(
            &data.dir,
            Some(&out),
            &FakeLiveness::default(),
            &wakeshot_windows::testing::MemoryLog::default(),
            &|_| {
                Err(WinError {
                    call: "SHOpenFolderAndSelectItems",
                    code: 5,
                })
            },
        );
        assert_eq!(answer.code, 0, "{answer:?}");
        assert!(
            answer
                .text
                .ends_with("(Explorer did not open: SHOpenFolderAndSelectItems failed with 5.)"),
            "{answer:?}"
        );
        assert_eq!(std::fs::read_dir(&out).unwrap().count(), 1);
    }
}
