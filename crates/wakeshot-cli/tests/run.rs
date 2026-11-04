//! The built `wakeshot.exe run`: exit codes, `--dry-run`, and Ctrl+C in a console of its own.
//!
//! Owner: sources track (`p4-cli-run`); fixes by `p6-int-dry-run-e2e`.
//!
//! Every run sets `WAKESHOT_DATA_DIR` (read by debug builds only) to a temp folder, so nothing touches
//! `%LocalAppData%\Wakeshot`. A release build skips every test that starts the exe: its exe ignores the variable.

use std::fs::File;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use wakeshot_core::holds::file::{Parsed, parse};
use wakeshot_core::process::{Alive, ProcessRef};
use wakeshot_windows::process::liveness;
use windows::Win32::System::Console::{
    CTRL_C_EVENT, GenerateConsoleCtrlEvent, GetConsoleProcessList, SetConsoleCtrlHandler,
};
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

const DATA_DIR: &str = "WAKESHOT_DATA_DIR";
const HELPER: &str = "WAKESHOT_TEST_CTRL_C_HELPER";
const SLEEPER: &str = "WAKESHOT_TEST_SLEEPER";
/// `STATUS_CONTROL_C_EXIT`: the code of a process the default Ctrl+C handler ended.
const CONTROL_C_EXIT: i32 = 0xC000_013A_u32 as i32;

/// `true` when this test must not run: a release exe ignores `WAKESHOT_DATA_DIR` and would use the real data folder.
fn release_build() -> bool {
    if cfg!(debug_assertions) {
        return false;
    }
    eprintln!("skipped: a release exe ignores {DATA_DIR} and would use the real data folder");
    true
}

/// A fresh folder under the temp folder, removed on drop.
struct Temp(PathBuf);

static NEXT: AtomicU32 = AtomicU32::new(0);

impl Temp {
    fn new(tag: &str) -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("wakeshot-run-e2e-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `wakeshot <before> <raw>`, with `raw` passed exactly as written.
fn wakeshot(data: &Path, before: &[&str], raw: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wakeshot"))
        .args(before)
        .raw_arg(raw)
        .env(DATA_DIR, data)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn json_files(folder: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_exe_exits_with_the_childs_whole_code() {
    if release_build() {
        return;
    }
    let data = Temp::new("codes");
    for (raw, want) in [
        ("cmd /d /c exit 7", 7),
        ("cmd /d /c exit 300", 300),
        ("cmd /d /c exit -1073741510", CONTROL_C_EXIT),
    ] {
        let out = wakeshot(&data.0, &["run", "--"], raw);
        assert_eq!(out.status.code(), Some(want), "{raw}");
        assert_eq!(
            String::from_utf8(out.stderr).unwrap(),
            "wakeshot: Wakeshot is not running, so nothing keeps the PC awake for this command until it starts.\n"
        );
    }
    assert_eq!(json_files(&data.0.join("holds")), Vec::<PathBuf>::new());
    let log = std::fs::read_to_string(data.0.join(r"logs\wakeshot.log")).unwrap();
    assert!(
        log.contains("cli: run: cmd exited with -1073741510 (0xC000013A)"),
        "{log}"
    );
}

#[test]
fn dry_run_echo_hi_prints_hi_and_uses_the_dry_run_folder() {
    if release_build() {
        return;
    }
    let data = Temp::new("dry-run");
    let out = wakeshot(&data.0, &["--dry-run", "run", "--"], "cmd /c echo hi");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "hi\r\n");
    let log = std::fs::read_to_string(data.0.join(r"dry-run\logs\wakeshot.log")).unwrap();
    assert!(log.contains("cli: run: cmd started, pid "), "{log}");
    assert!(log.contains("cli: run: cmd exited with 0"), "{log}");
    assert!(!data.0.join("logs").exists());
}

#[test]
fn quotes_and_spaces_reach_the_child_as_typed() {
    if release_build() {
        return;
    }
    let data = Temp::new("typed");
    let said = r#""a  b" c\d\"e "" \\x\\" end"#;
    let out = wakeshot(&data.0, &["run", "--"], &format!("cmd /d /c echo {said}"));
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("{said}\r\n")
    );
}

#[test]
fn a_program_not_found_exits_127() {
    if release_build() {
        return;
    }
    let data = Temp::new("missing");
    let out = wakeshot(&data.0, &["run", "--"], "wakeshot-test-no-such-program x");
    assert_eq!(out.status.code(), Some(127));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("'wakeshot-test-no-such-program' was not found in PATH."),
    );
}

#[test]
fn ctrl_c_ends_the_child_and_wakeshot_exits_with_its_code() {
    if release_build() {
        return;
    }
    let data = Temp::new("ctrl-c");
    // A console with no window of its own: Ctrl+C sent there reaches only the processes this test starts.
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "ctrl_c_helper",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(HELPER, &data.0)
        .creation_flags(CREATE_NO_WINDOW.0)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    // Shown with `--nocapture`: what the helper saw.
    println!("{text}");
    assert!(
        out.status.success(),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains(&format!("wakeshot exited with {CONTROL_C_EXIT}")),
        "{text}"
    );
}

/// Ends a child that is still running when a check fails, by its handle.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        if let Ok(None) = self.0.try_wait() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn wait_for(child: &mut Child, limit: Duration) -> Option<ExitStatus> {
    let deadline = Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return Some(status);
        }
        if Instant::now() > deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[ignore = "runs only inside ctrl_c_ends_the_child_and_wakeshot_exits_with_its_code, in its own console"]
fn ctrl_c_helper() {
    let Some(root) = std::env::var_os(HELPER).map(PathBuf::from) else {
        return;
    };
    // Normal Ctrl+C handling, whatever this process inherited: the ignore flag passes to children.
    // SAFETY: a plain call; the NULL handler form needs no function.
    unsafe { SetConsoleCtrlHandler(None, false) }.unwrap();
    let wakeshot = Command::new(env!("CARGO_BIN_EXE_wakeshot"))
        .args(["run", "--"])
        .arg(std::env::current_exe().unwrap())
        .args(["sleeper", "--exact", "--ignored", "--nocapture"])
        .env(DATA_DIR, &root)
        .env(SLEEPER, "1")
        .env_remove(HELPER)
        .stdin(Stdio::null())
        .stdout(File::create(root.join("out.txt")).unwrap())
        .stderr(File::create(root.join("err.txt")).unwrap())
        .spawn()
        .unwrap();
    let mut wakeshot = Running(wakeshot);

    // The hold file is written after the child starts and after wakeshot's handler is set.
    let deadline = Instant::now() + Duration::from_secs(15);
    let hold = loop {
        if let Some(path) = json_files(&root.join("holds")).pop() {
            break path;
        }
        assert!(Instant::now() < deadline, "no hold file within 15 s");
        thread::sleep(Duration::from_millis(20));
    };
    let name = hold.file_name().unwrap().to_string_lossy().into_owned();
    let file = match parse(&name, &std::fs::read(&hold).unwrap()) {
        Parsed::Ok(hold) => hold.file,
        other => panic!("{other:?}"),
    };
    let child = ProcessRef {
        pid: file.pid,
        start: file.proc_start.unwrap(),
    };
    assert_eq!(liveness(child), Alive::Yes);

    // Never send to a console that holds a process this test did not start.
    let mut attached = [0_u32; 16];
    // SAFETY: the call writes at most the array's length.
    let count = unsafe { GetConsoleProcessList(&mut attached) } as usize;
    let ours = [std::process::id(), wakeshot.0.id(), child.pid];
    assert!(
        (1..=attached.len()).contains(&count) && attached[..count].iter().all(|p| ours.contains(p)),
        "console shared with {:?}; ours {ours:?}",
        &attached[..count.min(attached.len())]
    );
    println!(
        "console holds {:?}: helper {}, wakeshot {}, child {}",
        &attached[..count],
        ours[0],
        ours[1],
        ours[2]
    );

    // Set after wakeshot started, so it does not inherit it.
    // SAFETY: plain calls; the NULL handler form needs no function.
    unsafe { SetConsoleCtrlHandler(None, true) }.unwrap();
    unsafe { GenerateConsoleCtrlEvent(CTRL_C_EVENT, 0) }.unwrap();
    let status = wait_for(&mut wakeshot.0, Duration::from_secs(15));
    let err = std::fs::read_to_string(root.join("err.txt")).unwrap_or_default();
    let status =
        status.unwrap_or_else(|| panic!("wakeshot still runs 15 s after Ctrl+C; stderr: {err}"));
    println!("wakeshot exited with {}", status.code().unwrap());
    assert_eq!(liveness(child), Alive::Gone, "the child still runs");
    // Both show that wakeshot outlived the Ctrl+C: the default handler would have ended it at once.
    assert!(!hold.exists(), "the hold file is still there");
    let log = std::fs::read_to_string(root.join(r"logs\wakeshot.log")).unwrap();
    assert!(
        log.contains("exited with -1073741510 (0xC000013A)"),
        "{log}"
    );
}

#[test]
#[ignore = "the child of ctrl_c_helper"]
fn sleeper() {
    if std::env::var_os(SLEEPER).is_some() {
        // Ends by itself if Ctrl+C never comes, so no process outlives the test for long.
        thread::sleep(Duration::from_secs(20));
    }
}
