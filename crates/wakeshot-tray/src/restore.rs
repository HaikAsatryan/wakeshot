//! Restore mode: restore at sign-in if a journal exists and its owner is dead (architecture section 4, "Processes").
//! Wiring only: the logic and its tests are in `wakeshot_windows::recovery`.

use std::process::ExitCode;

use wakeshot_windows::log::Proc;
use wakeshot_windows::power::journal::FileNet;
use wakeshot_windows::power::plan::Win32Power;
use wakeshot_windows::process::SystemLiveness;
use wakeshot_windows::recovery::{self, RecoveryDeps, Win32Recovery};
use wakeshot_windows::store::{AutoStart, DataDir, Names};

pub fn run() -> ExitCode {
    let Ok(dir) = DataDir::real() else {
        return ExitCode::from(recovery::EXIT_FAILED);
    };
    let log = recovery::open_log(&dir, Proc::Restore);
    let names = Names::real();
    let os = Win32Recovery::this_exe();
    let mut deps = RecoveryDeps {
        plans: Win32Power::new(log.clone()),
        // The real RunOnce value: Windows deleted it before running this, and the clean-up only ever clears it.
        net: FileNet::restore_only(dir, AutoStart::real(), log.clone()),
        names: &names,
        log: &*log,
        liveness: &SystemLiveness,
        os: &os,
    };
    ExitCode::from(recovery::run_restore_mode(&mut deps).exit_code())
}
