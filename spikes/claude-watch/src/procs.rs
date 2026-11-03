use windows::Win32::Foundation::{CloseHandle, FILETIME, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    WaitForSingleObject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    /// Running. The value is the creation time as a FILETIME number, the same unit as `procStart`.
    Running(u64),
    Exited,
    /// OpenProcess failed with this HRESULT: no such pid, or no access.
    NotOpen(i32),
}

pub fn check(pid: u32) -> Liveness {
    // SAFETY: plain Win32 calls on a handle owned here and closed before return.
    unsafe {
        let handle = match OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            false,
            pid,
        ) {
            Ok(h) => h,
            Err(e) => return Liveness::NotOpen(e.code().0),
        };
        // An exited process can still be opened while someone holds a handle to it, so ask whether it is signaled.
        let running = WaitForSingleObject(handle, 0) == WAIT_TIMEOUT;
        let mut created = FILETIME::default();
        let mut exited = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        let times = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user);
        let _ = CloseHandle(handle);
        match (running, times) {
            (true, Ok(())) => Liveness::Running(filetime_u64(created)),
            (true, Err(e)) => Liveness::NotOpen(e.code().0),
            (false, _) => Liveness::Exited,
        }
    }
}

pub fn filetime_u64(ft: FILETIME) -> u64 {
    (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
}

/// Rule 2 of `docs/research/claude-code-signals.md`: a status file counts only if its pid runs and the process
/// creation time equals `procStart`.
pub fn verdict(pid: u32, proc_start: Option<&str>) -> String {
    match check(pid) {
        Liveness::Running(created) => match proc_start.and_then(|s| s.parse::<u64>().ok()) {
            Some(s) if s == created => "alive, start time matches".to_string(),
            Some(_) => format!("alive, start time differs (pid reused, real start {created})"),
            None => "alive, no procStart to compare".to_string(),
        },
        Liveness::Exited => "dead (exited)".to_string(),
        Liveness::NotOpen(code) => format!("dead or no access (OpenProcess 0x{:08X})", code as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_process_matches_its_start_time() {
        let Liveness::Running(created) = check(std::process::id()) else {
            panic!("own process not running");
        };
        assert!(created > 0);
        let start = created.to_string();
        assert_eq!(
            verdict(std::process::id(), Some(&start)),
            "alive, start time matches"
        );
        assert!(verdict(std::process::id(), Some("1")).starts_with("alive, start time differs"));
    }

    #[test]
    fn exited_child_is_dead_even_while_its_handle_is_open() {
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/c", "exit 0"])
            .spawn()
            .expect("spawn cmd");
        child.wait().expect("wait");
        // `child` still holds the process handle here, so the pid cannot be reused yet.
        assert_eq!(check(child.id()), Liveness::Exited);
    }
}
