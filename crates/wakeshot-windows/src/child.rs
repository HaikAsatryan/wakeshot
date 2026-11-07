//! `wakeshot run`: find the program like PowerShell does, start it, and leave Ctrl+C to it.

use std::ffi::{OsStr, OsString};
use std::io;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::fs::MetadataExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;
use windows::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, SetConsoleCtrlHandler};
use windows::Win32::System::Environment::GetCommandLineW;
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::core::BOOL;

use crate::WinError;

/// `PATHEXT` when it is unset or lists nothing usable: cmd.exe's own default.
const DEFAULT_PATHEXT: [&str; 4] = [".COM", ".EXE", ".BAT", ".CMD"];

/// This process's command line as Windows passed it, before any splitting.
pub fn command_line() -> OsString {
    // SAFETY: the pointer is this process's own NUL-terminated command line, which lives as long as the process.
    unsafe { OsString::from_wide(GetCommandLineW().as_wide()) }
}

/// What a program token names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    /// A full path to an `.exe` or `.com` file: started directly.
    Exe(PathBuf),
    /// A full path to a `.bat` or `.cmd` file: started through `cmd.exe`.
    Batch(PathBuf),
    /// A file of another kind, by its extension as found, without the dot (`ps1`); empty for a file with none.
    NotRunnable(String),
    NotFound,
}

/// Finds a program like PowerShell does, from this process's `PATH` and `PATHEXT`.
pub fn resolve(token: &OsStr) -> Resolved {
    resolve_in(
        token,
        std::env::var_os("PATH").as_deref(),
        std::env::var_os("PATHEXT").as_deref(),
    )
}

/// [`resolve`] with `PATH` and `PATHEXT` given.
///
/// A token with a folder part (`\`, `/` or a drive) is used as given, made absolute against the current folder. A bare
/// name is searched in `PATH` only, never in the current folder, each folder in order. In each place the name is
/// tried as written when it has an extension, then with each `PATHEXT` extension in order (unless its extension is
/// one of them), so `npm` finds `npm.cmd` and never the extensionless shell script beside it.
pub fn resolve_in(token: &OsStr, path: Option<&OsStr>, pathext: Option<&OsStr>) -> Resolved {
    if token.is_empty() {
        return Resolved::NotFound;
    }
    let extensions = extensions(pathext);
    if has_folder(token) {
        let Ok(full) = std::path::absolute(token) else {
            return Resolved::NotFound;
        };
        return match find(&full, &extensions) {
            Some(found) => classify(found),
            // A file without an extension, named on purpose: saying what it is helps more than "not found".
            None if is_file(&full) => classify(full),
            None => Resolved::NotFound,
        };
    }
    let folders = path.map(std::env::split_paths).into_iter().flatten();
    // A relative entry would search the current folder, which a bare name never does.
    folders
        .filter(|folder| folder.is_absolute())
        .find_map(|folder| find(&folder.join(token), &extensions))
        .map_or(Resolved::NotFound, classify)
}

fn extensions(pathext: Option<&OsStr>) -> Vec<String> {
    let listed: Vec<String> = pathext
        .map(|p| {
            p.to_string_lossy()
                .split(';')
                .filter(|e| e.len() > 1 && e.starts_with('.'))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    match listed.is_empty() {
        true => DEFAULT_PATHEXT.iter().map(|e| (*e).to_owned()).collect(),
        false => listed,
    }
}

fn has_folder(token: &OsStr) -> bool {
    token
        .as_encoded_bytes()
        .iter()
        .any(|b| matches!(b, b'\\' | b'/' | b':'))
}

/// The first file of: `base` itself when it has an extension, then `base` plus each extension in order.
fn find(base: &Path, extensions: &[String]) -> Option<PathBuf> {
    let own = base.extension();
    let listed = own
        .and_then(OsStr::to_str)
        .is_some_and(|own| extensions.iter().any(|e| e[1..].eq_ignore_ascii_case(own)));
    let as_written = own.is_some().then(|| base.to_path_buf());
    let with_extension = extensions.iter().filter(|_| !listed).map(|extension| {
        let mut name = base.as_os_str().to_owned();
        name.push(extension);
        PathBuf::from(name)
    });
    as_written
        .into_iter()
        .chain(with_extension)
        .find(|candidate| is_file(candidate))
}

/// The attributes without following a link, as std's own program search reads them: a link counts even when its
/// target cannot be opened. App execution aliases (`WindowsApps\winget.exe`) are such links.
fn is_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .is_ok_and(|meta| meta.file_attributes() & FILE_ATTRIBUTE_DIRECTORY.0 == 0)
}

fn classify(path: PathBuf) -> Resolved {
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    match extension.to_ascii_lowercase().as_str() {
        "exe" | "com" => Resolved::Exe(path),
        "bat" | "cmd" => Resolved::Batch(path),
        _ => Resolved::NotRunnable(extension),
    }
}

/// The command [`start`] spawns; `None` for a program it does not start. `rest` is the child's text after its
/// program, exactly as typed.
///
/// The program is the resolved full path, so std's own search, which ignores `PATHEXT`, never runs. A batch file goes
/// through `%ComSpec% /d /s /c ""<full path>" <rest>"`: the full path instead of the name as typed, so cmd.exe runs
/// the file found here and does not search the current folder first; `/s` makes cmd.exe drop only the outer quotes.
pub fn command(resolved: &Resolved, rest: &OsStr) -> Option<Command> {
    let command = match resolved {
        Resolved::Exe(path) => {
            let mut command = Command::new(path);
            if !rest.is_empty() {
                command.raw_arg(rest);
            }
            command
        }
        Resolved::Batch(path) => {
            let mut line = OsString::from("/d /s /c \"\"");
            line.push(path);
            line.push("\"");
            if !rest.is_empty() {
                line.push(" ");
                line.push(rest);
            }
            line.push("\"");
            let mut command = Command::new(command_prompt());
            command.raw_arg(line);
            command
        }
        Resolved::NotRunnable(_) | Resolved::NotFound => return None,
    };
    Some(command)
}

/// Starts the child in this console with this process's standard handles. No new process group: that would turn off
/// Ctrl+C in the child.
pub fn start(resolved: &Resolved, rest: &OsStr) -> io::Result<Child> {
    command(resolved, rest)
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?
        .spawn()
}

/// `%ComSpec%` when it is a full path, as Node and Python use it; else `cmd.exe` in the system folder.
fn command_prompt() -> PathBuf {
    if let Some(spec) = std::env::var_os("ComSpec")
        .map(PathBuf::from)
        .filter(|spec| spec.is_absolute())
    {
        return spec;
    }
    let mut buffer = [0_u16; 260];
    // SAFETY: the buffer is a local array; the call writes at most its length.
    let length = unsafe { GetSystemDirectoryW(Some(&mut buffer)) } as usize;
    match length {
        // A failed call leaves the name for std's search, which looks in the system folder too.
        0 => PathBuf::from("cmd.exe"),
        length if length >= buffer.len() => PathBuf::from("cmd.exe"),
        length => PathBuf::from(OsString::from_wide(&buffer[..length])).join("cmd.exe"),
    }
}

/// Leaves Ctrl+C and Ctrl+Break to the child: this process lives on, to delete the hold file and exit with the
/// child's code. A handler, never the NULL form, which every child started later would inherit.
pub fn ignore_ctrl_c() -> Result<(), WinError> {
    // SAFETY: the handler is a plain function that lives as long as the process.
    unsafe { SetConsoleCtrlHandler(Some(on_console_event), true) }.map_err(|e| WinError {
        call: "SetConsoleCtrlHandler",
        code: win32_code(&e),
    })
}

/// Closing the console, sign-out and shutdown go on to the default handler, which ends this process as usual.
unsafe extern "system" fn on_console_event(kind: u32) -> BOOL {
    BOOL::from(kind == CTRL_C_EVENT || kind == CTRL_BREAK_EVENT)
}

/// The Win32 code inside an HRESULT made from one, else the HRESULT.
fn win32_code(e: &windows::core::Error) -> u32 {
    let hresult = e.code().0 as u32;
    match hresult & 0xFFFF_0000 {
        0x8007_0000 => hresult & 0xFFFF,
        _ => hresult,
    }
}

#[cfg(test)]
mod tests {
    use std::process::Stdio;
    use std::sync::atomic::{AtomicU32, Ordering};

    use windows::Win32::System::Console::{
        CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT,
    };

    use super::*;

    /// A fresh folder under the temp folder, with a space in its name, removed on drop.
    struct Scratch(PathBuf);

    static NEXT: AtomicU32 = AtomicU32::new(0);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("wakeshot child {tag} {}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn folder(&self, name: &str) -> PathBuf {
            let folder = self.0.join(name);
            std::fs::create_dir_all(&folder).unwrap();
            folder
        }

        fn file(&self, folder: &str, name: &str, text: &str) -> PathBuf {
            let path = self.folder(folder).join(name);
            std::fs::write(&path, text).unwrap();
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn joined(folders: &[&Path]) -> OsString {
        std::env::join_paths(folders).unwrap()
    }

    const STANDARD: &str = ".COM;.EXE;.BAT;.CMD;.VBS;.JS";

    fn exe(path: PathBuf) -> Resolved {
        Resolved::Exe(path)
    }

    #[test]
    fn a_bare_name_takes_the_first_folder_with_a_pathext_match() {
        let s = Scratch::new("path");
        // Decoys first: an extensionless shell script (npm ships one beside npm.cmd) and a PowerShell script.
        let first = s.folder("first");
        s.file("first", "tool", "#!/bin/sh");
        s.file("first", "tool.ps1", "");
        let second = s.folder("second");
        s.file("second", "tool.cmd", "@echo off");
        let third = s.folder("third");
        s.file("third", "tool.exe", "");
        let path = joined(&[&first, &second, &third]);
        let ext = Some(OsStr::new(STANDARD));
        assert_eq!(
            resolve_in(OsStr::new("tool"), Some(&path), ext),
            Resolved::Batch(second.join("tool.CMD"))
        );
        let path = joined(&[&first, &third, &second]);
        assert_eq!(
            resolve_in(OsStr::new("tool"), Some(&path), ext),
            exe(third.join("tool.EXE"))
        );
    }

    #[test]
    fn pathext_order_decides_inside_one_folder() {
        let s = Scratch::new("order");
        let bin = s.folder("bin");
        s.file("bin", "tool.cmd", "");
        s.file("bin", "tool.exe", "");
        let path = joined(&[&bin]);
        assert_eq!(
            resolve_in(OsStr::new("tool"), Some(&path), Some(OsStr::new(STANDARD))),
            exe(bin.join("tool.EXE"))
        );
        assert_eq!(
            resolve_in(
                OsStr::new("tool"),
                Some(&path),
                Some(OsStr::new(".cmd;.exe"))
            ),
            Resolved::Batch(bin.join("tool.cmd"))
        );
    }

    #[test]
    fn a_name_with_its_extension_is_taken_as_written() {
        let s = Scratch::new("written");
        let bin = s.folder("bin");
        s.file("bin", "tool.cmd", "");
        s.file("bin", "tool.exe", "");
        s.file("bin", "deploy.ps1", "");
        s.file("bin", "old.com", "");
        s.file("bin", "job.BAT", "");
        s.file("bin", "python3.12.exe", "");
        let path = joined(&[&bin]);
        let ext = Some(OsStr::new(STANDARD));
        let found = |token: &str| resolve_in(OsStr::new(token), Some(&path), ext);
        assert_eq!(found("tool.cmd"), Resolved::Batch(bin.join("tool.cmd")));
        assert_eq!(found("TOOL.EXE"), exe(bin.join("TOOL.EXE")));
        assert_eq!(found("old.com"), exe(bin.join("old.com")));
        assert_eq!(found("job.bat"), Resolved::Batch(bin.join("job.bat")));
        assert_eq!(found("deploy.ps1"), Resolved::NotRunnable("ps1".into()));
        assert_eq!(found("deploy"), Resolved::NotFound);
        // An extension that is not a PATHEXT one is a dot in the name: the extensions are still tried.
        assert_eq!(found("python3.12"), exe(bin.join("python3.12.EXE")));
        assert_eq!(found("tool.js"), Resolved::NotFound);
    }

    #[test]
    fn the_current_folder_is_not_searched() {
        // Tests run in the package folder, which holds Cargo.toml.
        assert!(Path::new("Cargo.toml").is_file());
        let ext = Some(OsStr::new(".TOML"));
        let empty = OsString::new();
        for path in [None, Some(empty.as_os_str()), Some(OsStr::new(".;src"))] {
            assert_eq!(
                resolve_in(OsStr::new("Cargo.toml"), path, ext),
                Resolved::NotFound
            );
            assert_eq!(
                resolve_in(OsStr::new("Cargo"), path, ext),
                Resolved::NotFound
            );
        }
        // Named with its folder, it is found.
        let here = std::path::absolute("Cargo.toml").unwrap();
        assert_eq!(here, std::env::current_dir().unwrap().join("Cargo.toml"));
        assert_eq!(
            resolve_in(OsStr::new(r".\Cargo.toml"), None, ext),
            Resolved::NotRunnable("toml".into())
        );
        assert_eq!(
            resolve_in(OsStr::new(r".\Cargo"), None, ext),
            Resolved::NotRunnable("TOML".into())
        );
    }

    #[test]
    fn a_token_with_a_folder_is_used_as_given() {
        let s = Scratch::new("folder");
        let bin = s.folder("bin");
        s.file("bin", "tool.cmd", "");
        s.file("bin", "script", "");
        let ext = Some(OsStr::new(STANDARD));
        let other = s.folder("other");
        s.file("other", "tool.exe", "");
        let path = joined(&[&other]);
        let at = |token: &Path| resolve_in(token.as_os_str(), Some(&path), ext);
        assert_eq!(at(&bin.join("tool")), Resolved::Batch(bin.join("tool.CMD")));
        assert_eq!(
            at(&bin.join("tool.cmd")),
            Resolved::Batch(bin.join("tool.cmd"))
        );
        assert_eq!(
            at(&bin.join("script")),
            Resolved::NotRunnable(String::new())
        );
        assert_eq!(at(&bin.join("missing.exe")), Resolved::NotFound);
        assert_eq!(at(&bin.join("missing")), Resolved::NotFound);
        let slashes = PathBuf::from(bin.to_string_lossy().replace('\\', "/")).join("tool");
        assert!(matches!(at(&slashes), Resolved::Batch(_)));
    }

    #[test]
    fn folders_are_skipped_and_quoted_path_entries_read() {
        let s = Scratch::new("dirs");
        let bin = s.folder("bin");
        s.folder("bin/tool.exe");
        let semi = s.folder("with;semi");
        s.file("with;semi", "tool.cmd", "");
        let mut path = OsString::from("\"");
        path.push(&semi);
        path.push("\";;");
        path.push(&bin);
        assert_eq!(
            resolve_in(OsStr::new("tool"), Some(&path), Some(OsStr::new(STANDARD))),
            Resolved::Batch(semi.join("tool.CMD"))
        );
        let only_bin = joined(&[&bin]);
        assert_eq!(
            resolve_in(
                OsStr::new("tool"),
                Some(&only_bin),
                Some(OsStr::new(STANDARD))
            ),
            Resolved::NotFound
        );
        assert_eq!(
            resolve_in(OsStr::new(""), Some(&only_bin), None),
            Resolved::NotFound
        );
    }

    #[test]
    fn an_empty_or_odd_pathext_falls_back_to_the_default() {
        let want: Vec<String> = DEFAULT_PATHEXT.iter().map(|e| (*e).to_owned()).collect();
        assert_eq!(extensions(None), want);
        assert_eq!(extensions(Some(OsStr::new(""))), want);
        assert_eq!(extensions(Some(OsStr::new(";.;exe;"))), want);
        assert_eq!(extensions(Some(OsStr::new(".CMD;;.EXE"))), [".CMD", ".EXE"]);
    }

    #[test]
    fn the_command_passes_the_rest_unchanged() {
        let rest = OsStr::new(r#"a  "b c" d\"e"#);
        let tool = PathBuf::from(r"C:\Program Files\x\tool.exe");
        let command = command(&exe(tool.clone()), rest).unwrap();
        assert_eq!(command.get_program(), tool.as_os_str());
        assert_eq!(command.get_args().collect::<Vec<_>>(), [rest]);
        let bare = super::command(&exe(tool), OsStr::new("")).unwrap();
        assert_eq!(bare.get_args().count(), 0);

        let script = PathBuf::from(r"C:\Program Files\nodejs\npm.cmd");
        let batch = super::command(&Resolved::Batch(script), rest).unwrap();
        assert_eq!(batch.get_program(), command_prompt().as_os_str());
        assert_eq!(
            batch.get_args().collect::<Vec<_>>(),
            [OsStr::new(
                r#"/d /s /c ""C:\Program Files\nodejs\npm.cmd" a  "b c" d\"e""#
            )]
        );
        assert!(super::command(&Resolved::NotFound, rest).is_none());
        assert!(super::command(&Resolved::NotRunnable("ps1".into()), rest).is_none());
    }

    #[test]
    fn the_command_prompt_is_a_full_path_to_cmd() {
        let prompt = command_prompt();
        assert!(prompt.is_absolute(), "{prompt:?}");
        assert!(prompt.is_file(), "{prompt:?}");
    }

    /// What an `.exe` child gets: `cmd /u /c echo` prints the raw text after `echo ` (UTF-16 with `/u`).
    #[test]
    fn an_exe_child_gets_the_text_exactly() {
        let cmd = resolve(OsStr::new("cmd"));
        assert!(matches!(cmd, Resolved::Exe(_)), "{cmd:?}");
        let said = "\"a  b\" c\\d\\\"e \"\" \\\\x\\\\\" \t tab \u{532}\u{561}\u{580}\u{587} \u{1F600} -- end";
        let rest = format!("/u /d /c echo {said}");
        let out = command(&cmd, OsStr::new(&rest))
            .unwrap()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .output()
            .unwrap();
        assert!(out.status.success());
        let (pairs, odd) = out.stdout.as_chunks::<2>();
        assert!(odd.is_empty());
        let units: Vec<u16> = pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
        assert_eq!(String::from_utf16(&units).unwrap(), format!("{said}\r\n"));
    }

    /// What a `.cmd` child gets: `%*` is its arguments as typed, `%1` the first one.
    #[test]
    fn a_batch_child_sees_its_arguments_as_typed() {
        let s = Scratch::new("batch");
        let script = s.file(
            "bin dir",
            "show args.cmd",
            "@echo off\r\n>\"%~dp0seen.txt\" echo(%*\r\n>>\"%~dp0seen.txt\" echo(%1\r\n>>\"%~dp0seen.txt\" echo(%~2\r\nexit /b 3\r\n",
        );
        let rest = r#""a  b" "c \"d\" e" f\g --"#;
        let mut child = command(&Resolved::Batch(script.clone()), OsStr::new(rest))
            .unwrap()
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        let status = child.wait().unwrap();
        assert_eq!(status.code(), Some(3));
        let seen = std::fs::read_to_string(script.with_file_name("seen.txt")).unwrap();
        assert_eq!(seen, format!("{rest}\r\n\"a  b\"\r\nc \\\"d\\\" e\r\n"));
    }

    #[test]
    fn start_starts_and_refuses_what_it_cannot_run() {
        let cmd = resolve(OsStr::new("cmd"));
        let mut child = start(&cmd, OsStr::new("/d /c exit 7")).unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(7));
        let mut child = start(&cmd, OsStr::new("/d /c exit -1073741510")).unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(0xC000_013A_u32 as i32));
        assert_eq!(
            start(&Resolved::NotFound, OsStr::new(""))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn the_handler_takes_only_ctrl_c_and_ctrl_break() {
        for (kind, taken) in [
            (CTRL_C_EVENT, true),
            (CTRL_BREAK_EVENT, true),
            (CTRL_CLOSE_EVENT, false),
            (CTRL_LOGOFF_EVENT, false),
            (CTRL_SHUTDOWN_EVENT, false),
        ] {
            // SAFETY: the handler only compares its argument.
            assert_eq!(unsafe { on_console_event(kind) }.as_bool(), taken, "{kind}");
        }
    }

    #[test]
    fn the_command_line_names_this_test_program() {
        let line = command_line();
        let exe = std::env::current_exe().unwrap();
        let name = exe.file_stem().unwrap().to_string_lossy().into_owned();
        assert!(line.to_string_lossy().contains(&name), "{line:?}");
    }
}
