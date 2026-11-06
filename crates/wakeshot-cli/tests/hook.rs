//! The built `wakeshot.exe hook`, as an agent's hook runs it: `{}` on stdout, nothing on stderr, exit 0, for every form,
//! every input and a panic (`docs/agent-hooks.md`, "The hook command", step 6 and "The `wakeshot hook` command form is
//! frozen"); and no write through a junction, elevated or at Medium (risk 19).
//!
//! Every run sets `WAKESHOT_DATA_DIR` (read by debug builds only) to a temp folder, so nothing touches
//! `%LocalAppData%\Wakeshot`; a release build skips the tests that need it.
//!
//! Owner: `p4-cursor-source` (first version from `p4-contracts-r3`).

use std::ffi::OsString;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

use wakeshot_windows::process::{self, spawn_outside_tree};

const DATA_DIR: &str = "WAKESHOT_DATA_DIR";
const PANIC_ENV: &str = "WAKESHOT_TEST_HOOK_PANIC";
const CHAT: &str = "a3f1c2d4-5b6e-4f70-8a9b-0c1d2e3f4a5b";
const TURN: &str = "7d2e9c41-0b8a-4e55-9f13-6a2c4b7d8e90";
const MARKER_WORDS: [&str; 3] = ["QXV7", "marker", "77317731"];

fn wakeshot(data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wakeshot"))
        .args(args)
        .env(DATA_DIR, data)
        .output()
        .unwrap()
}

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
        let root = std::env::temp_dir().join(format!(
            "wakeshot-hook-e2e-{tag}-{}-{n}",
            std::process::id()
        ));
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

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("wakeshot-core")
        .join("tests")
        .join("fixtures")
        .join("cursor")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Runs `exe` with `input` on a pipe written by a thread, and gives the output and whether the whole input went in:
/// a hook that stops reading would break the agent's pipe.
fn run_with_input(mut command: Command, input: Vec<u8>) -> (Output, bool) {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let writer = thread::spawn(move || stdin.write_all(&input).is_ok());
    let output = child.wait_with_output().unwrap();
    (output, writer.join().unwrap())
}

fn hook(data: &Path, event: &str, input: Vec<u8>) -> (Output, bool) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wakeshot"));
    command.args(["hook", "cursor", event]).env(DATA_DIR, data);
    run_with_input(command, input)
}

fn assert_answered(output: &Output, case: &str) {
    assert_eq!(output.status.code(), Some(0), "{case}");
    assert_eq!(output.stdout, b"{}", "{case}");
    assert!(
        output.stderr.is_empty(),
        "{case}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Every file under `root` with its bytes.
fn files(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(files(&path));
        } else {
            let bytes = std::fs::read(&path).unwrap_or_default();
            out.push((path, bytes));
        }
    }
    out
}

/// `hook cursor remove` is left out: at Medium it removes the real profile's Cursor plugin (`agent_hooks`), which no
/// test may touch; the unit tests check its call.
#[test]
fn every_hook_form_answers_an_empty_object_with_exit_0() {
    if release_build() {
        return;
    }
    let data = Temp::new("forms");
    for args in [
        &["hook", "cursor", "beforeSubmitPrompt"][..],
        &["hook", "cursor", "stop"],
        &["hook", "cursor", "sessionEnd"],
        &["hook", "nobody", "remove"],
        &["hook", "antigravity", "PreInvocation"],
        &["--dry-run", "hook", "cursor", "stop"],
        &[
            "hook",
            "cursor",
            "beforeSubmitPrompt",
            "--checked",
            "a",
            "b",
        ],
        // Forms no version knows: still the answer, never help or an error.
        &["hook"],
        &["hook", "--help"],
        &["hook", "cursor", "stop", "--dry-run"],
        &["hook", "nobody", "nothing", "extra"],
        &["HOOK", "Cursor", "Stop"],
    ] {
        let out = wakeshot(&data.0, args);
        assert_answered(&out, &format!("{args:?}"));
    }
}

#[test]
fn every_input_answers_an_empty_object_with_exit_0_and_is_read_to_its_end() {
    if release_build() {
        return;
    }
    let data = Temp::new("inputs");
    let mut with_bom = vec![0xEF, 0xBB, 0xBF];
    with_bom.extend(fixture("editor-before-submit-prompt.json"));
    // Past the 64 MiB the hook reads, so the rest must be drained.
    let mut huge = format!(r#"{{"conversation_id":"{CHAT}","prompt":""#).into_bytes();
    huge.resize(65 * 1024 * 1024, b'a');
    huge.extend_from_slice(b"\"}");
    let inputs: Vec<(&str, Vec<u8>)> = vec![
        ("prompt", fixture("editor-before-submit-prompt.json")),
        ("stop", fixture("editor-stop-completed.json")),
        ("end", fixture("editor-session-end.json")),
        ("cli", fixture("cli-before-submit-prompt.json")),
        ("bom", with_bom),
        ("empty", Vec::new()),
        ("garbage", b"not json at all".to_vec()),
        ("bytes", (0..=255u8).cycle().take(10_000).collect()),
        ("array", b"[1,2,3]".to_vec()),
        ("deep", "[".repeat(100_000).into_bytes()),
        ("huge", huge),
    ];
    for (case, input) in inputs {
        for event in ["beforeSubmitPrompt", "stop", "sessionEnd"] {
            let (output, all_read) = hook(&data.0, event, input.clone());
            assert_answered(&output, &format!("{case} as {event}"));
            assert!(all_read, "{case} as {event}: the pipe was read to its end");
        }
    }
    // No Cursor runs above this test, so the hook wrote nothing but maybe a log line.
    let written: Vec<_> = files(&data.0)
        .into_iter()
        .map(|(path, _)| path)
        .filter(|path| path.extension().is_none_or(|e| e != "log"))
        .collect();
    assert!(written.is_empty(), "{written:?}");
}

#[test]
fn a_panic_still_answers_an_empty_object_with_exit_0_and_says_nothing() {
    if release_build() {
        return;
    }
    let data = Temp::new("panic");
    let input = String::from_utf8(fixture("editor-before-submit-prompt.json"))
        .unwrap()
        .replace("Refactor the parser", "QXV7 marker");
    let mut command = Command::new(env!("CARGO_BIN_EXE_wakeshot"));
    command
        .args(["hook", "cursor", "beforeSubmitPrompt"])
        .env(DATA_DIR, &data.0)
        .env(PANIC_ENV, "1");
    let (output, all_read) = run_with_input(command, input.into_bytes());
    assert_answered(&output, "a panic");
    assert!(all_read);
    for (path, bytes) in files(&data.0) {
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            MARKER_WORDS.iter().all(|w| !text.contains(w)),
            "{}",
            path.display()
        );
    }
}

#[test]
fn the_marker_never_reaches_stdout_stderr_or_the_log() {
    if release_build() {
        return;
    }
    let data = Temp::new("marker");
    let base = String::from_utf8(fixture("editor-before-submit-prompt.json")).unwrap();
    for input in [
        base.replace("Refactor the parser and add tests", "QXV7 marker"),
        base.replace("A3F1C2D4-5B6E-4F70-8A9B-0C1D2E3F4A5B", "QXV7 marker"),
        base.replace("\"3.27.12\"", "77317731"),
        base.replace("\"beforeSubmitPrompt\"", "{\"QXV7 marker\":77317731}"),
    ] {
        let (output, _) = hook(&data.0, "beforeSubmitPrompt", input.into_bytes());
        assert_answered(&output, "marker");
    }
    for (path, bytes) in files(&data.0) {
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            MARKER_WORDS.iter().all(|w| !text.contains(w)),
            "{}",
            path.display()
        );
    }
}

/// A copy of `cmd.exe` named `Cursor.exe`, so the hook's walk finds the editor's image above it.
fn fake_cursor(dir: &Path) -> PathBuf {
    let system =
        std::env::var_os("SystemRoot").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    let exe = dir.join("Cursor.exe");
    std::fs::copy(system.join("System32").join("cmd.exe"), &exe).unwrap();
    exe
}

/// Runs the hook as Cursor would: under `Cursor.exe`, the input from a file on its stdin. Gives the fake Cursor's
/// process, whose pid and start the state file must name.
fn hook_under_cursor(
    cursor: &Path,
    data: &Path,
    event: &str,
    input: &[u8],
) -> wakeshot_core::process::ProcessRef {
    let input_file = data.with_extension(format!("{event}.json"));
    std::fs::write(&input_file, input).unwrap();
    let line = format!(
        r#"/d /s /c ""{}" hook cursor {event} < "{}"""#,
        env!("CARGO_BIN_EXE_wakeshot"),
        input_file.display()
    );
    let mut child = Command::new(cursor)
        .raw_arg(line)
        .env(DATA_DIR, data)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let process = wakeshot_core::process::ProcessRef {
        pid: child.id(),
        start: process::start_of(&child).unwrap(),
    };
    assert!(child.wait().unwrap().success());
    let _ = std::fs::remove_file(&input_file);
    process
}

fn chat_file(data: &Path) -> PathBuf {
    data.join("agents")
        .join("cursor")
        .join(format!("{CHAT}.json"))
}

/// The whole chain on this PC: a hook under `Cursor.exe` writes the chat's file naming that process, and its turn's
/// stop deletes it. Elevated, the write comes from the Medium copy started under Explorer, which also shows that the
/// copy gets this process's environment (the test data folder).
#[test]
#[ignore = "needs Explorer as the shell when the runner is elevated (track note 3)"]
fn a_hook_under_cursor_writes_the_chats_file_and_its_stop_deletes_it() {
    if release_build() {
        return;
    }
    let temp = Temp::new("chain");
    let data = temp.0.join("data");
    let cursor = fake_cursor(&temp.0);
    let process = hook_under_cursor(
        &cursor,
        &data,
        "beforeSubmitPrompt",
        &fixture("editor-before-submit-prompt.json"),
    );
    let text = std::fs::read_to_string(chat_file(&data)).expect("the chat's file");
    assert_eq!(
        text,
        format!(
            r#"{{"version":1,"pid":{},"procStart":"{}","generation":"{TURN}","label":"Cursor","state":"working"}}"#,
            process.pid, process.start
        )
    );
    hook_under_cursor(
        &cursor,
        &data,
        "stop",
        &fixture("editor-stop-completed.json"),
    );
    assert!(!chat_file(&data).exists());
}

/// Gives `dir` a High mandatory label, so a Medium process cannot write in it and a High one can.
fn label_high(dir: &Path) {
    let out = Command::new("icacls")
        .arg(dir)
        .args(["/setintegritylevel", "(OI)(CI)H"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

fn junction(link: &Path, target: &Path) {
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    let out = Command::new("cmd")
        .raw_arg(format!(
            r#"/d /c mklink /J "{}" "{}""#,
            link.display(),
            target.display()
        ))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

fn elevated() -> bool {
    let above = process::current_above_medium().unwrap();
    if !above {
        eprintln!("skipped: the High label needs an elevated runner");
    }
    above
}

/// An elevated hook under a junction in place of `agents\cursor` (step 4, risk 19): it writes nothing itself, and its
/// Medium copy (or, without Explorer, its still elevated copy, which refuses) writes nothing through the link into a
/// folder Medium may not write. A hook that wrote while elevated would put the file there.
#[test]
#[ignore = "needs Explorer as the shell for the Medium copy (track note 3)"]
fn an_elevated_hook_writes_nothing_through_a_junction() {
    if release_build() || !elevated() {
        return;
    }
    let temp = Temp::new("junction-high");
    let data = temp.0.join("data");
    let target = temp.0.join("target");
    std::fs::create_dir_all(&target).unwrap();
    label_high(&target);
    junction(&data.join("agents").join("cursor"), &target);
    // A file the stop of this turn would delete, and the prompt would replace.
    let victim = target.join(format!("{CHAT}.json"));
    std::fs::write(&victim, victim_text()).unwrap();
    let cursor = fake_cursor(&temp.0);
    for (event, input) in [
        ("beforeSubmitPrompt", "editor-before-submit-prompt.json"),
        ("stop", "editor-stop-completed.json"),
    ] {
        hook_under_cursor(&cursor, &data, event, &fixture(input));
        assert_eq!(
            files(&target),
            [(victim.clone(), victim_text())],
            "{event}: nothing written or deleted through the link"
        );
    }
}

/// A chat file of this test's turn that no hook wrote.
fn victim_text() -> Vec<u8> {
    format!(
        r#"{{"version":1,"pid":1,"procStart":"1","generation":"{TURN}","label":"Cursor","state":"working"}}"#
    )
    .into_bytes()
}

/// The Medium copy itself, started under Explorer with checked values as an elevated hook starts it: with a plain
/// folder it writes the chat's file; with a junction to a folder Medium may not write, nothing lands there. It runs
/// unelevated, so the deny is the one a Medium process meets (`CLAUDE.md`, elevation note).
#[test]
#[ignore = "needs Explorer as the shell (track note 3)"]
fn the_medium_copy_writes_where_it_may_and_nothing_through_a_junction() {
    if release_build() || !elevated() {
        return;
    }
    let temp = Temp::new("junction-medium");
    let me = process::current().unwrap();
    let deadline = (wakeshot_windows::clock::now().wall_ms + 60_000).to_string();
    let run_copy = |data: &Path| {
        let values = [
            "hook",
            "cursor",
            "beforeSubmitPrompt",
            "--checked",
            CHAT,
            TURN,
            &me.pid.to_string(),
            &me.start.to_string(),
            "editor",
            "working",
            &deadline,
        ];
        let args: Vec<OsString> = values.iter().map(OsString::from).collect();
        set_data_dir(data);
        let spawned = spawn_outside_tree(Path::new(env!("CARGO_BIN_EXE_wakeshot")), &args).unwrap();
        assert!(spawned.outside_tree, "started under Explorer");
        assert!(spawned.handle.wait(10_000).unwrap(), "the copy ended");
    };

    let plain = temp.0.join("plain");
    run_copy(&plain);
    assert!(
        chat_file(&plain).exists(),
        "the Medium copy writes a plain folder"
    );

    let data = temp.0.join("linked");
    let target = temp.0.join("target");
    std::fs::create_dir_all(&target).unwrap();
    label_high(&target);
    let victim = target.join(format!("{CHAT}.json"));
    std::fs::write(&victim, victim_text()).unwrap();
    junction(&data.join("agents").join("cursor"), &target);
    run_copy(&data);
    assert_eq!(
        files(&target),
        [(victim, victim_text())],
        "nothing written through the link"
    );
    // Denied for the right reason: the Medium write itself, not a check that never ran.
    let log = std::fs::read_to_string(data.join("logs").join("wakeshot.log")).unwrap();
    assert!(
        log.contains(
            "cursor beforeSubmitPrompt: the chat's file did not change: CreateFileW failed with 5"
        ),
        "{log}"
    );
}

/// `spawn_outside_tree` takes no environment, so its child gets this process's: the data folder reaches the copy
/// this way.
fn set_data_dir(data: &Path) {
    // SAFETY: std takes its own lock for every environment read and write, and nothing in this test binary reads the
    // environment outside std.
    unsafe { std::env::set_var(DATA_DIR, data) };
}
