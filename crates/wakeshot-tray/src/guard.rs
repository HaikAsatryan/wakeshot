//! Guard mode: wait on the tray and restore if it dies (architecture section 5, "Guard").
//!
//! Owner: power track (`p3-recovery`). Wiring only: the logic and its tests are in `wakeshot_windows::recovery`.

use std::ffi::OsString;
use std::process::ExitCode;

use wakeshot_windows::log::{Level, Proc};
use wakeshot_windows::power::journal::FileNet;
use wakeshot_windows::power::plan::Win32Power;
use wakeshot_windows::process::SystemLiveness;
use wakeshot_windows::recovery::{self, GuardArgs, RecoveryDeps, Win32Recovery};
use wakeshot_windows::store::{AutoStart, DataDir, Names};

/// `args`: what follows `--guard` (the tray's pid, start time and the stop event).
pub fn run(args: &[OsString]) -> ExitCode {
    let Ok(dir) = DataDir::real() else {
        return ExitCode::from(recovery::EXIT_FAILED);
    };
    let log = recovery::open_log(&dir, Proc::Guard);
    let args = match GuardArgs::parse(args) {
        Ok(args) => args,
        Err(why) => {
            let text = format!("guard: bad command line: {why}");
            log.line(Level::Error, "recovery", &text);
            return ExitCode::from(recovery::EXIT_BAD_ARGS);
        }
    };
    let names = Names::real();
    let os = Win32Recovery::this_exe();
    let mut deps = RecoveryDeps {
        plans: Win32Power::new(log.clone()),
        // The real RunOnce value: the restore's clean-up only ever clears it.
        net: FileNet::restore_only(dir, AutoStart::real(), log.clone()),
        names: &names,
        log: &*log,
        liveness: &SystemLiveness,
        os: &os,
    };
    ExitCode::from(recovery::run_guard(&args, &mut deps).exit_code())
}
