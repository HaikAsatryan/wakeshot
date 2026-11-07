//! Open a file or an `ms-settings:` URI with the default app, or show a file in Explorer.
//!
//! The tray opens the log and the first-run Settings page with it, `wakeshot log` opens the log, and `wakeshot
//! diagnostics` shows its bundle.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_INVALID_PARAMETER, ERROR_TIMEOUT};
use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows::Win32::UI::Shell::{
    ILCreateFromPathW, ILFree, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
    SHOpenFolderAndSelectItems, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow, SW_SHOWNORMAL};
use windows::core::{PCWSTR, w};

use crate::WinError;

/// How long the caller waits for the shell. A handler that takes longer finishes on its own thread, so the tray's IO
/// thread never waits on it for more than this.
pub const OPEN_WAIT_MS: u64 = 10_000;

/// No Win32 error uses it: the mark of a shell thread that ended without an answer.
const NO_WIN32_CODE: u32 = u32::MAX;

/// Opens `target`, a file path or a URI such as `ms-settings:taskbar`, as a double click would (`ShellExecuteExW`,
/// verb "open"). Returns once the shell has started the app, or with `ERROR_TIMEOUT` after [`OPEN_WAIT_MS`].
///
/// The call runs on a short thread of its own with COM initialized apartment-threaded, as Microsoft asks for the
/// shell, so the caller's thread needs no COM setup. Windows shows no error dialog: the caller reports the error.
pub fn open_with_default_app(target: &OsStr) -> Result<(), WinError> {
    let target = wide(target)?;
    let (answer, wait) = mpsc::channel();
    std::thread::Builder::new()
        .name("wakeshot-shell".to_owned())
        .spawn(move || {
            // The caller may have stopped waiting; then nobody needs the answer.
            let _ = answer.send(open_here(&target));
        })
        .map_err(|e| WinError {
            call: "CreateThread",
            code: e.raw_os_error().map_or(NO_WIN32_CODE, |c| c as u32),
        })?;
    match wait.recv_timeout(Duration::from_millis(OPEN_WAIT_MS)) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err(WinError {
            call: "ShellExecuteExW",
            code: ERROR_TIMEOUT.0,
        }),
        Err(RecvTimeoutError::Disconnected) => Err(WinError {
            call: "ShellExecuteExW",
            code: NO_WIN32_CODE,
        }),
    }
}

/// Opens an Explorer window on `file`'s folder with `file` selected (`SHOpenFolderAndSelectItems`), for the
/// diagnostics bundle. Same thread, COM and wait as [`open_with_default_app`]: Microsoft asks for COM before the call.
pub fn show_in_folder(file: &Path) -> Result<(), WinError> {
    let target = wide(file.as_os_str())?;
    let (answer, wait) = mpsc::channel();
    std::thread::Builder::new()
        .name("wakeshot-shell".to_owned())
        .spawn(move || {
            let _ = answer.send(select_here(&target));
        })
        .map_err(|e| WinError {
            call: "CreateThread",
            code: e.raw_os_error().map_or(NO_WIN32_CODE, |c| c as u32),
        })?;
    match wait.recv_timeout(Duration::from_millis(OPEN_WAIT_MS)) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err(WinError {
            call: "SHOpenFolderAndSelectItems",
            code: ERROR_TIMEOUT.0,
        }),
        Err(RecvTimeoutError::Disconnected) => Err(WinError {
            call: "SHOpenFolderAndSelectItems",
            code: NO_WIN32_CODE,
        }),
    }
}

/// `target` is NUL-terminated.
fn select_here(target: &[u16]) -> Result<(), WinError> {
    // SAFETY: a new thread with no COM yet; no reserved pointer.
    let init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    if init.is_err() {
        return Err(WinError {
            call: "CoInitializeEx",
            code: init.0 as u32,
        });
    }
    // SAFETY: as in `open_here`.
    let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };
    // SAFETY: `target` is NUL-terminated and lives for the call.
    let item = unsafe { ILCreateFromPathW(PCWSTR(target.as_ptr())) };
    let shown = if item.is_null() {
        Err(WinError {
            call: "ILCreateFromPathW",
            code: ERROR_FILE_NOT_FOUND.0,
        })
    } else {
        // A count of 0 with the item's full id list opens its folder and selects it (Microsoft).
        // SAFETY: `item` is a live id list from the shell.
        let shown = unsafe { SHOpenFolderAndSelectItems(item, None, 0) }
            .map_err(|e| win32_of("SHOpenFolderAndSelectItems", &e));
        // SAFETY: `item` came from `ILCreateFromPathW` and is freed once, after its last use.
        unsafe { ILFree(Some(item)) };
        shown
    };
    // SAFETY: pairs the successful `CoInitializeEx` above, on the same thread.
    unsafe { CoUninitialize() };
    shown
}

/// NUL-terminated; a NUL inside would cut the name short and open something else.
fn wide(target: &OsStr) -> Result<Vec<u16>, WinError> {
    let mut wide: Vec<u16> = target.encode_wide().collect();
    if wide.is_empty() || wide.contains(&0) {
        return Err(WinError {
            call: "ShellExecuteExW",
            code: ERROR_INVALID_PARAMETER.0,
        });
    }
    wide.push(0);
    Ok(wide)
}

/// `target` is NUL-terminated.
fn open_here(target: &[u16]) -> Result<(), WinError> {
    // Microsoft, ShellExecuteEx Remarks: shell extensions may need a single-threaded apartment, set up this way.
    // SAFETY: a new thread with no COM yet; no reserved pointer.
    let init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    if init.is_err() {
        return Err(WinError {
            call: "CoInitializeEx",
            code: init.0 as u32,
        });
    }
    // Lets the opened app come to the front. It works only while this process may set the foreground itself (the
    // tray right after a menu click); otherwise it fails and the app opens behind, which is no reason to fail.
    // SAFETY: takes a value and returns nothing else.
    let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        // NOASYNC: this thread ends right after the call, so the shell must finish here (Microsoft,
        // SHELLEXECUTEINFO Remarks). NO_UI: an error comes back to the caller instead of a dialog that a CLI exit
        // would close.
        fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpVerb: w!("open"),
        lpFile: PCWSTR(target.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // SAFETY: `info` and the strings it points to live for the call.
    let opened = unsafe { ShellExecuteExW(&mut info) };
    // SAFETY: pairs the successful `CoInitializeEx` above, on the same thread.
    unsafe { CoUninitialize() };
    opened.map_err(|e| win32_of("ShellExecuteExW", &e))
}

/// The Win32 code when the HRESULT wraps one, as `store` and `log` report codes.
fn win32_of(call: &'static str, e: &windows::core::Error) -> WinError {
    let hresult = e.code().0 as u32;
    let code = match hresult & 0xFFFF_0000 == 0x8007_0000 {
        true => hresult & 0xFFFF,
        false => hresult,
    };
    WinError { call, code }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Instant;

    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND};

    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wakeshot-shell-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_nul_or_an_empty_target_opens_nothing() {
        let refused = Err(WinError {
            call: "ShellExecuteExW",
            code: ERROR_INVALID_PARAMETER.0,
        });
        assert_eq!(open_with_default_app(OsStr::new("a\0b.log")), refused);
        assert_eq!(open_with_default_app(OsStr::new("")), refused);
    }

    /// The whole path, thread and COM included, with a target no app can open: a folder that does not exist and an
    /// extension nothing is registered for. With no error dialog, nothing shows on the screen.
    #[test]
    fn a_missing_file_is_an_error_without_a_dialog() {
        let missing = scratch("missing").join("missing.wakeshot-test-none");
        let result = open_with_default_app(missing.as_os_str());
        let Err(e) = result else {
            panic!("opened {missing:?}");
        };
        assert_eq!(e.call, "ShellExecuteExW");
        assert!(
            [ERROR_FILE_NOT_FOUND.0, ERROR_PATH_NOT_FOUND.0].contains(&e.code),
            "{e:?}"
        );
    }

    /// The real open, run once by hand (`--ignored`): a temp `.cmd` file opens in its default app, writes a mark
    /// beside itself and ends, so its window closes on its own. A file of the user's editor type would land in an
    /// editor window the user may have open with unsaved work.
    #[test]
    #[ignore = "opens a window: run once by hand with a person at the screen"]
    fn real_open_file() {
        let dir = scratch("real");
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("open-check.cmd");
        let mark = dir.join("opened.txt");
        std::fs::write(
            &script,
            "@echo opened> \"%~dp0opened.txt\"\r\n@exit /b 0\r\n",
        )
        .unwrap();
        let started = Instant::now();
        assert_eq!(open_with_default_app(script.as_os_str()), Ok(()));
        let returned_ms = started.elapsed().as_millis();
        while !mark.exists() && started.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(100));
        }
        let ran_ms = started.elapsed().as_millis();
        assert!(mark.exists(), "the script did not run within 10 s");
        println!("open returned after {returned_ms} ms; the script ran within {ran_ms} ms");
        // The script may still be ending; its window closes itself.
        std::thread::sleep(Duration::from_secs(1));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
