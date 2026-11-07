//! The command line: parsing, help and errors, and the raw text of the command after `run --`.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::process::ExitCode;

use wakeshot_core::view::KeepAwake;

use crate::command::{Command, Global};

const USAGE: &str = "\
Wakeshot keeps this PC awake while something needs it.

Usage: wakeshot <command>

Commands:
  run -- <command>                           Run a command, awake until it ends
  keep-awake 30m|1h|2h|4h|on|off [--screen]  Keep awake for a time, or until off
  automatic on|off                           Watch AI agents and 'run' commands
  status                                     Show what Wakeshot is doing
  restore                                    Restore your power settings
  log                                        Open the log
  diagnostics [folder]                       Save a file for a bug report

'keep-awake on' lasts until you turn it off. '--screen' keeps the screen on too.

Test aid:
  --dry-run   Before the command: use the dry-run data folder, as the tray's
              --dry-run does. Such as: wakeshot --dry-run status
";

const DRY_RUN: &str = "--dry-run";
const RUN: &str = "run";
const HOOK: &str = "hook";
const CHECKED: &str = "--checked";
/// After a command as well as in its place, so `wakeshot status --help` helps instead of failing.
const HELP_FLAGS: [&str; 3] = ["--help", "-h", "/?"];
const LENGTHS: &str = "30m, 1h, 2h, 4h, on or off";
const RUN_FORM: &str = "such as 'wakeshot run -- npm test'";

/// `args` is the command line without the program name, as `std::env::args_os` gives it.
///
/// Words match in any ASCII case, as Windows' own commands do. Everything after `run --` belongs to the child and is
/// never read here.
pub fn parse(args: &[OsString]) -> (Global, Command) {
    let mut global = Global { dry_run: false };
    let mut at = 0;
    while args.get(at).is_some_and(|a| is(a, DRY_RUN)) {
        global.dry_run = true;
        at += 1;
    }
    let command = match args.get(at) {
        None => Command::Help,
        Some(word) => command(word, &args[at + 1..]),
    };
    (global, command)
}

fn command(word: &OsStr, rest: &[OsString]) -> Command {
    // Before every help and error form: an agent runs it, and `Bad` exits 2, which blocks the agent's prompt.
    if is(word, HOOK) {
        return hook(rest);
    }
    if is(word, "help") || is_help_flag(word) {
        return Command::Help;
    }
    if is(word, RUN) {
        return run(rest);
    }
    if rest.iter().any(|a| is_help_flag(a)) {
        return Command::Help;
    }
    if rest.iter().any(|a| is(a, DRY_RUN)) {
        return late_dry_run();
    }
    match word.to_str().map(str::to_ascii_lowercase).as_deref() {
        Some("keep-awake") => keep_awake(rest),
        Some("automatic") => automatic(rest),
        Some("status") => alone(rest, "status", Command::Status),
        Some("restore") => alone(rest, "restore", Command::Restore),
        Some("log") => alone(rest, "log", Command::Log),
        Some("diagnostics") => diagnostics(rest),
        _ if is_option(word) => wrong(format!("unknown option '{}'", shown(word))),
        _ => wrong(format!("unknown command '{}'", shown(word))),
    }
}

/// Only the shape is checked: the child's text comes from the raw command line through [`run_tail`], so nothing is
/// re-quoted.
fn run(rest: &[OsString]) -> Command {
    match rest.first() {
        Some(a) if a == "--" => {}
        Some(a) if is_help_flag(a) => return Command::Help,
        Some(a) if is(a, DRY_RUN) => return late_dry_run(),
        _ => return wrong(format!("'run' needs '--' before the command, {RUN_FORM}")),
    }
    match rest.get(1) {
        None => wrong(format!("'run' needs a command after '--', {RUN_FORM}")),
        Some(program) if program.is_empty() => wrong("the command after '--' is empty"),
        Some(_) => Command::Run,
    }
}

fn keep_awake(rest: &[OsString]) -> Command {
    let mut choice = None;
    let mut screen = false;
    for arg in rest {
        if is(arg, "--screen") {
            screen = true;
            continue;
        }
        let Some(length) = length(arg) else {
            return wrong(match is_option(arg) {
                true => format!("'keep-awake' has no option '{}'", shown(arg)),
                false => format!("'{}' is not a length; use {LENGTHS}", shown(arg)),
            });
        };
        if choice.replace(length).is_some() {
            return wrong("'keep-awake' takes one length");
        }
    }
    match choice {
        None => wrong(format!("'keep-awake' needs a length: {LENGTHS}")),
        Some(KeepAwake::Off) if screen => wrong("'--screen' does not go with 'off'"),
        Some(choice) => Command::KeepAwake { choice, screen },
    }
}

/// The menu's lengths only: a script that wants another one writes its own hold file (`docs/hold-files.md`).
fn length(arg: &OsStr) -> Option<KeepAwake> {
    let lower = arg.to_str()?.to_ascii_lowercase();
    Some(match lower.as_str() {
        "30m" => KeepAwake::Minutes(30),
        "1h" => KeepAwake::Minutes(60),
        "2h" => KeepAwake::Minutes(120),
        "4h" => KeepAwake::Minutes(240),
        "on" => KeepAwake::UntilOff,
        "off" => KeepAwake::Off,
        _ => return None,
    })
}

fn automatic(rest: &[OsString]) -> Command {
    match rest {
        [word] if is(word, "on") => Command::Automatic(true),
        [word] if is(word, "off") => Command::Automatic(false),
        [word] => wrong(format!(
            "'automatic' takes on or off, not '{}'",
            shown(word)
        )),
        _ => wrong("'automatic' needs on or off"),
    }
}

/// `hook <agent> <event> [--checked <value>...]`, the frozen form (`Command::Hook`). The agent and event are kept as
/// given, case and all: they are the agent's own names. A word that is not Unicode is kept as best it can be, so it
/// matches no table and no value check.
fn hook(rest: &[OsString]) -> Command {
    let text = |word: &OsString| word.to_string_lossy().into_owned();
    let word = |at: usize| rest.get(at).map(text).unwrap_or_default();
    let values = match rest.get(2..) {
        None | Some([]) => Vec::new(),
        Some([flag, values @ ..]) if is(flag, CHECKED) && !values.is_empty() => {
            values.iter().map(text).collect()
        }
        Some(_) => {
            return Command::Hook {
                agent: String::new(),
                event: String::new(),
                values: Vec::new(),
            };
        }
    };
    Command::Hook {
        agent: word(0),
        event: word(1),
        values,
    }
}

/// `diagnostics [folder]`: the Desktop without a folder. The folder is kept as given; the command checks it exists.
fn diagnostics(rest: &[OsString]) -> Command {
    match rest {
        [] => Command::Diagnostics(None),
        [option] if is_option(option) => {
            wrong(format!("'diagnostics' has no option '{}'", shown(option)))
        }
        [folder] => Command::Diagnostics(Some(folder.clone())),
        _ => wrong("'diagnostics' takes one folder at most"),
    }
}

fn alone(rest: &[OsString], name: &str, command: Command) -> Command {
    match rest.first() {
        None => command,
        Some(_) => wrong(format!("'{name}' takes no arguments")),
    }
}

fn late_dry_run() -> Command {
    wrong("put '--dry-run' before the command, such as 'wakeshot --dry-run status'")
}

fn wrong(why: impl Into<String>) -> Command {
    Command::Bad(why.into())
}

fn is(arg: &OsStr, word: &str) -> bool {
    arg.to_str().is_some_and(|a| a.eq_ignore_ascii_case(word))
}

fn is_help_flag(arg: &OsStr) -> bool {
    HELP_FLAGS.iter().any(|flag| is(arg, flag))
}

fn is_option(arg: &OsStr) -> bool {
    arg.as_encoded_bytes().first() == Some(&b'-')
}

fn shown(arg: &OsStr) -> String {
    arg.to_string_lossy().into_owned()
}

// Write errors are ignored here and below: `println!` would panic on a closed pipe, and the release build aborts on
// panic.
pub fn help() -> ExitCode {
    let _ = std::io::stdout().write_all(USAGE.as_bytes());
    ExitCode::SUCCESS
}

pub fn bad(why: &str) -> ExitCode {
    let _ = writeln!(std::io::stderr(), "wakeshot: {why}. Run 'wakeshot --help'.");
    ExitCode::from(2)
}

const SPACE: u16 = b' ' as u16;
const TAB: u16 = b'\t' as u16;
const QUOTE: u16 = b'"' as u16;
const BACKSLASH: u16 = b'\\' as u16;

/// The child's part of the raw command line (`GetCommandLineW`): everything after the `--` that follows `run`, from
/// its first argument to the end, exactly as typed. `None` when the line is not `wakeshot [--dry-run] run -- <...>`.
///
/// The line is split by the rules `std::env::args_os` uses (the MSVC C runtime's, `docs/research.md` "Command line
/// splitting and the built CLI"), so this finds the same `run` and `--` that [`parse`] saw.
pub fn run_tail(raw: &OsStr) -> Option<OsString> {
    let units: Vec<u16> = raw.encode_wide().collect();
    let mut at = program_end(&units);
    loop {
        let (word, end) = next_arg(&units, at)?;
        at = end;
        let word = OsString::from_wide(&word);
        if is(&word, DRY_RUN) {
            continue;
        }
        if !is(&word, RUN) {
            return None;
        }
        break;
    }
    let (dashes, end) = next_arg(&units, at)?;
    if dashes != [b'-' as u16; 2] {
        return None;
    }
    let start = skip_blanks(&units, end);
    (start < units.len()).then(|| OsString::from_wide(&units[start..]))
}

/// The child's program as `args_os` reads it, and the raw text after it with the blanks between them dropped.
/// `None` for a tail with no argument at all.
pub fn split_program(tail: &OsStr) -> Option<(OsString, OsString)> {
    let units: Vec<u16> = tail.encode_wide().collect();
    let (program, end) = next_arg(&units, 0)?;
    let rest = skip_blanks(&units, end);
    Some((
        OsString::from_wide(&program),
        OsString::from_wide(&units[rest..]),
    ))
}

/// Where the program name ends. Its own rule: a quote only toggles quoting, with no escapes, since a path cannot
/// hold a quote.
fn program_end(units: &[u16]) -> usize {
    let mut in_quotes = false;
    for (at, &unit) in units.iter().enumerate() {
        match unit {
            QUOTE => in_quotes = !in_quotes,
            SPACE | TAB if !in_quotes => return at,
            _ => {}
        }
    }
    units.len()
}

fn skip_blanks(units: &[u16], from: usize) -> usize {
    from + units[from..]
        .iter()
        .take_while(|&&u| u == SPACE || u == TAB)
        .count()
}

/// One argument from `from` on (blanks first skipped): its value and the index just past it. A port of std's
/// `parse_lp_cmd_line` loop (Rust 1.99.1, `library/std/src/sys/args/windows.rs`), one argument at a time.
fn next_arg(units: &[u16], from: usize) -> Option<(Vec<u16>, usize)> {
    let mut at = skip_blanks(units, from);
    if at == units.len() {
        return None;
    }
    let mut value = Vec::new();
    let mut in_quotes = false;
    while let Some(&unit) = units.get(at) {
        match unit {
            SPACE | TAB if !in_quotes => break,
            BACKSLASH => {
                let count = units[at..].iter().take_while(|&&u| u == BACKSLASH).count();
                at += count;
                if units.get(at) == Some(&QUOTE) {
                    value.extend(std::iter::repeat_n(BACKSLASH, count / 2));
                    if count % 2 == 1 {
                        value.push(QUOTE);
                        at += 1;
                    }
                } else {
                    value.extend(std::iter::repeat_n(BACKSLASH, count));
                }
                continue;
            }
            QUOTE if in_quotes => match units.get(at + 1) {
                Some(&QUOTE) => {
                    value.push(QUOTE);
                    at += 1;
                }
                Some(_) => in_quotes = false,
                None => {}
            },
            QUOTE => in_quotes = true,
            other => value.push(other),
        }
        at += 1;
    }
    Some((value, at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    fn parsed(list: &[&str]) -> Command {
        parse(&args(list)).1
    }

    fn bad_text(list: &[&str]) -> String {
        match parsed(list) {
            Command::Bad(why) => why,
            other => panic!("{list:?}: {other:?}"),
        }
    }

    fn tail(raw: &str) -> Option<String> {
        run_tail(OsStr::new(raw)).map(|t| t.into_string().unwrap())
    }

    const PLAIN: Global = Global { dry_run: false };
    const DRY: Global = Global { dry_run: true };

    #[test]
    fn help_for_no_arguments_and_every_help_form() {
        assert_eq!(parse(&args(&[])), (PLAIN, Command::Help));
        for form in ["--help", "-h", "/?", "help", "HELP", "--Help"] {
            assert_eq!(parse(&args(&[form])), (PLAIN, Command::Help), "{form}");
        }
        assert_eq!(parse(&args(&["--dry-run"])), (DRY, Command::Help));
        assert_eq!(parsed(&["help", "status"]), Command::Help);
        assert_eq!(parsed(&["status", "--help"]), Command::Help);
        assert_eq!(parsed(&["keep-awake", "-h"]), Command::Help);
        assert_eq!(parsed(&["run", "--help"]), Command::Help);
    }

    #[test]
    fn every_command() {
        assert_eq!(parsed(&["run", "--", "npm", "test"]), Command::Run);
        assert_eq!(parsed(&["run", "--", "x"]), Command::Run);
        assert_eq!(parsed(&["automatic", "on"]), Command::Automatic(true));
        assert_eq!(parsed(&["automatic", "off"]), Command::Automatic(false));
        assert_eq!(parsed(&["status"]), Command::Status);
        assert_eq!(parsed(&["restore"]), Command::Restore);
        assert_eq!(parsed(&["log"]), Command::Log);
        assert_eq!(parsed(&["diagnostics"]), Command::Diagnostics(None));
        assert_eq!(
            parsed(&["Diagnostics", "D:/Bug Reports"]),
            Command::Diagnostics(Some(OsString::from("D:/Bug Reports")))
        );
    }

    #[test]
    fn diagnostics_takes_one_folder() {
        assert_eq!(
            bad_text(&["diagnostics", "a", "b"]),
            "'diagnostics' takes one folder at most"
        );
        assert_eq!(
            bad_text(&["diagnostics", "--zip"]),
            "'diagnostics' has no option '--zip'"
        );
        assert_eq!(parsed(&["diagnostics", "--help"]), Command::Help);
        assert!(matches!(
            parsed(&["diagnostics", "--dry-run"]),
            Command::Bad(_)
        ));
        assert_eq!(
            parse(&args(&["--dry-run", "diagnostics"])),
            (DRY, Command::Diagnostics(None))
        );
    }

    #[test]
    fn keep_awake_lengths_and_screen() {
        for (word, choice) in [
            ("30m", KeepAwake::Minutes(30)),
            ("1h", KeepAwake::Minutes(60)),
            ("2h", KeepAwake::Minutes(120)),
            ("4h", KeepAwake::Minutes(240)),
            ("on", KeepAwake::UntilOff),
            ("off", KeepAwake::Off),
        ] {
            let screen = false;
            assert_eq!(
                parsed(&["keep-awake", word]),
                Command::KeepAwake { choice, screen },
                "{word}"
            );
            if choice != KeepAwake::Off {
                let screen = true;
                assert_eq!(
                    parsed(&["keep-awake", word, "--screen"]),
                    Command::KeepAwake { choice, screen }
                );
                assert_eq!(
                    parsed(&["keep-awake", "--screen", word]),
                    Command::KeepAwake { choice, screen }
                );
            }
        }
    }

    #[test]
    fn words_match_in_any_case() {
        assert_eq!(parsed(&["STATUS"]), Command::Status);
        assert_eq!(
            parsed(&["Keep-Awake", "1H", "--SCREEN"]),
            Command::KeepAwake {
                choice: KeepAwake::Minutes(60),
                screen: true
            }
        );
        assert_eq!(parsed(&["Automatic", "OFF"]), Command::Automatic(false));
        assert_eq!(
            parse(&args(&["--DRY-RUN", "Run", "--", "x"])),
            (DRY, Command::Run)
        );
    }

    #[test]
    fn dry_run_goes_before_the_command() {
        assert_eq!(
            parse(&args(&["--dry-run", "status"])),
            (DRY, Command::Status)
        );
        assert_eq!(
            parse(&args(&["--dry-run", "--dry-run", "log"])),
            (DRY, Command::Log)
        );
        assert_eq!(
            parse(&args(&[
                "--dry-run",
                "run",
                "--",
                "cmd",
                "/c",
                "echo",
                "hi"
            ])),
            (DRY, Command::Run)
        );
        let late = "put '--dry-run' before the command, such as 'wakeshot --dry-run status'";
        for list in [
            &["status", "--dry-run"][..],
            &["keep-awake", "1h", "--dry-run"],
            &["automatic", "--dry-run", "on"],
            &["restore", "--dry-run"],
            &["log", "--dry-run"],
            &["run", "--dry-run", "--", "x"],
        ] {
            assert_eq!(bad_text(list), late, "{list:?}");
        }
        // After `run --` it is the child's own argument.
        assert_eq!(
            parse(&args(&["run", "--", "tool", "--dry-run"])),
            (PLAIN, Command::Run)
        );
    }

    #[test]
    fn run_errors_show_the_right_form() {
        let dashes = "'run' needs '--' before the command, such as 'wakeshot run -- npm test'";
        assert_eq!(bad_text(&["run"]), dashes);
        assert_eq!(bad_text(&["run", "npm", "test"]), dashes);
        assert_eq!(bad_text(&["run", "-", "npm"]), dashes);
        assert_eq!(
            bad_text(&["run", "--"]),
            "'run' needs a command after '--', such as 'wakeshot run -- npm test'"
        );
        assert_eq!(
            bad_text(&["run", "--", ""]),
            "the command after '--' is empty"
        );
    }

    #[test]
    fn keep_awake_errors() {
        let needs = "'keep-awake' needs a length: 30m, 1h, 2h, 4h, on or off";
        assert_eq!(bad_text(&["keep-awake"]), needs);
        assert_eq!(bad_text(&["keep-awake", "--screen"]), needs);
        assert_eq!(
            bad_text(&["keep-awake", "3h"]),
            "'3h' is not a length; use 30m, 1h, 2h, 4h, on or off"
        );
        assert_eq!(
            bad_text(&["keep-awake", "60"]),
            "'60' is not a length; use 30m, 1h, 2h, 4h, on or off"
        );
        assert_eq!(
            bad_text(&["keep-awake", "1h", "--scren"]),
            "'keep-awake' has no option '--scren'"
        );
        assert_eq!(
            bad_text(&["keep-awake", "1h", "2h"]),
            "'keep-awake' takes one length"
        );
        assert_eq!(
            bad_text(&["keep-awake", "off", "--screen"]),
            "'--screen' does not go with 'off'"
        );
        assert_eq!(
            bad_text(&["keep-awake", "--screen", "off"]),
            "'--screen' does not go with 'off'"
        );
    }

    #[test]
    fn other_errors() {
        assert_eq!(bad_text(&["nonsense"]), "unknown command 'nonsense'");
        assert_eq!(
            bad_text(&["--verbose", "status"]),
            "unknown option '--verbose'"
        );
        assert_eq!(bad_text(&["-x"]), "unknown option '-x'");
        assert_eq!(bad_text(&["automatic"]), "'automatic' needs on or off");
        assert_eq!(
            bad_text(&["automatic", "on", "off"]),
            "'automatic' needs on or off"
        );
        assert_eq!(
            bad_text(&["automatic", "yes"]),
            "'automatic' takes on or off, not 'yes'"
        );
        assert_eq!(bad_text(&["status", "now"]), "'status' takes no arguments");
        assert_eq!(
            bad_text(&["restore", "all"]),
            "'restore' takes no arguments"
        );
        assert_eq!(bad_text(&["log", "x"]), "'log' takes no arguments");
        // A word that is not Unicode is shown as best it can be, never a panic.
        let odd = OsString::from_wide(&[b'x' as u16, 0xD800]);
        assert_eq!(
            parse(&[odd]).1,
            Command::Bad("unknown command 'x\u{FFFD}'".into())
        );
    }

    fn hook(agent: &str, event: &str, values: &[&str]) -> Command {
        Command::Hook {
            agent: agent.into(),
            event: event.into(),
            values: values.iter().map(|v| (*v).to_owned()).collect(),
        }
    }

    #[test]
    fn the_hook_form_and_its_checked_values() {
        assert_eq!(
            parsed(&["hook", "cursor", "beforeSubmitPrompt"]),
            hook("cursor", "beforeSubmitPrompt", &[])
        );
        assert_eq!(
            parsed(&["HOOK", "Cursor", "STOP"]),
            hook("Cursor", "STOP", &[])
        );
        assert_eq!(
            parse(&args(&["--dry-run", "hook", "cursor", "remove"])),
            (DRY, hook("cursor", "remove", &[]))
        );
        let checked = [
            "4f1c0a2e-0000-4000-8000-000000000001",
            "gen_1",
            "23816",
            "134381211391959336",
            "Cursor",
            "working",
            "134381211411959336",
        ];
        let mut list = vec!["hook", "cursor", "beforeSubmitPrompt", "--checked"];
        list.extend(checked);
        assert_eq!(
            parsed(&list),
            hook("cursor", "beforeSubmitPrompt", &checked)
        );
        // Values are the copy's to check, so any text passes the parse, `--help` and `..` too.
        assert_eq!(
            parsed(&[
                "hook",
                "cursor",
                "stop",
                "--Checked",
                r"..\..\x",
                "--help",
                ""
            ]),
            hook("cursor", "stop", &[r"..\..\x", "--help", ""])
        );
    }

    /// An agent runs it, so no shape may give `Help` or `Bad` (exit 2 blocks Cursor's prompt): a missing word is empty,
    /// any other shape gives empty words, which no agent table knows.
    #[test]
    fn every_hook_shape_is_a_hook_never_help_or_an_error() {
        assert_eq!(parsed(&["hook"]), hook("", "", &[]));
        assert_eq!(parsed(&["hook", "cursor"]), hook("cursor", "", &[]));
        for list in [
            &["hook", "--help"][..],
            &["hook", "-h", "x"],
            &["hook", "/?", "x", "y"],
            &["hook", "--dry-run"],
        ] {
            let Command::Hook { values, .. } = parsed(list) else {
                panic!("{list:?}: {:?}", parsed(list));
            };
            assert!(values.is_empty(), "{list:?}");
        }
        for list in [
            &["hook", "cursor", "stop", "extra"][..],
            &["hook", "cursor", "stop", "--dry-run"],
            &["hook", "cursor", "stop", "--help"],
            &["hook", "cursor", "stop", "--checked"],
            &["hook", "cursor", "stop", "x", "--checked", "y"],
        ] {
            assert_eq!(parsed(list), hook("", "", &[]), "{list:?}");
        }
        let odd = OsString::from_wide(&[b'x' as u16, 0xD800]);
        assert_eq!(
            parse(&[OsString::from("hook"), odd, OsString::from("stop")]).1,
            hook("x\u{FFFD}", "stop", &[])
        );
    }

    #[test]
    fn help_names_every_command_and_the_test_aid() {
        for word in [
            "run -- <command>",
            "keep-awake 30m|1h|2h|4h|on|off [--screen]",
            "automatic on|off",
            "status",
            "restore",
            "log",
            "diagnostics [folder]",
            "--dry-run",
        ] {
            assert!(USAGE.contains(word), "{word}");
        }
        assert!(USAGE.lines().all(|line| line.chars().count() <= 80));
        // An agent's command, not the user's.
        assert!(!USAGE.contains(HOOK) && !USAGE.contains(CHECKED));
    }

    #[test]
    fn run_tail_is_the_text_after_run_dashes() {
        assert_eq!(
            tail("wakeshot run -- npm test").as_deref(),
            Some("npm test")
        );
        assert_eq!(
            tail("wakeshot --dry-run run -- cmd /c echo hi").as_deref(),
            Some("cmd /c echo hi")
        );
        assert_eq!(tail("wakeshot run --    x  ").as_deref(), Some("x  "));
        assert_eq!(tail("WAKESHOT RUN -- x").as_deref(), Some("x"));
    }

    #[test]
    fn run_tail_with_a_program_path_with_spaces() {
        assert_eq!(
            tail(r#""C:\Program Files\Wakeshot\wakeshot.exe" run -- npm test"#).as_deref(),
            Some("npm test")
        );
        // The program name's own rule: a quote toggles, a backslash never escapes it.
        assert_eq!(
            tail(r#"C:\x\"a b"\wakeshot.exe run -- tool"#).as_deref(),
            Some("tool")
        );
        // A quote that closes in the middle does not end the name: the name runs to the next blank.
        assert_eq!(tail(r#""C:\a b\wakeshot.exe"run -- x"#), None);
    }

    #[test]
    fn run_tail_keeps_quoted_paths_and_spaces_as_typed() {
        let child = r#""C:\Program Files\nodejs\npm.cmd" run "build  it" '--x'"#;
        assert_eq!(
            tail(&format!("wakeshot run -- {child}")).as_deref(),
            Some(child)
        );
    }

    #[test]
    fn run_tail_reads_dashes_and_run_through_quotes() {
        assert_eq!(tail(r#"wakeshot run "--" x"#).as_deref(), Some("x"));
        assert_eq!(tail(r#"wakeshot "run" -- x"#).as_deref(), Some("x"));
        assert_eq!(
            tail(r#"wakeshot "--dry-run" run -- x"#).as_deref(),
            Some("x")
        );
        // `--` inside a quoted argument is not the separator.
        assert_eq!(tail(r#"wakeshot run "-- x""#), None);
        assert_eq!(tail(r#"wakeshot "run --" x"#), None);
        // An empty quoted part joins the next text, so `""--` is `--`, as `args_os` reads it.
        assert_eq!(tail(r#"wakeshot run ""-- x"#).as_deref(), Some("x"));
    }

    #[test]
    fn run_tail_leaves_a_later_dashes_to_the_child() {
        assert_eq!(
            tail("wakeshot run -- git log -- src").as_deref(),
            Some("git log -- src")
        );
        assert_eq!(tail("wakeshot run -- -- x").as_deref(), Some("-- x"));
        assert_eq!(
            tail(r#"wakeshot run -- tool "a -- b" --"#).as_deref(),
            Some(r#"tool "a -- b" --"#)
        );
    }

    #[test]
    fn run_tail_with_backslashes_and_quotes() {
        let child = r#"tool "a\"b" c:\dir\ "d\\" e\\\"f "g""h" \\server\share"#;
        assert_eq!(
            tail(&format!("wakeshot run -- {child}")).as_deref(),
            Some(child)
        );
        // An escaped quote before `--` keeps it out of the separator's reach.
        assert_eq!(tail(r#"wakeshot run \"-- x"#), None);
        assert_eq!(tail(r#"wakeshot run --\" x"#), None);
    }

    #[test]
    fn run_tail_with_tabs() {
        assert_eq!(
            tail("wakeshot\trun\t--\t\tcmd\t/c \t exit").as_deref(),
            Some("cmd\t/c \t exit")
        );
    }

    #[test]
    fn run_tail_keeps_non_ascii_and_lone_surrogates() {
        let child = "echo \u{532}\u{561}\u{580}\u{587} \u{1F600}";
        assert_eq!(
            tail(&format!("wakeshot run -- {child}")).as_deref(),
            Some(child)
        );
        let mut raw: Vec<u16> = "wakeshot run -- x".encode_utf16().collect();
        raw.push(0xDC00);
        let want = OsString::from_wide(&[b'x' as u16, 0xDC00]);
        assert_eq!(run_tail(&OsString::from_wide(&raw)), Some(want));
    }

    #[test]
    fn run_tail_without_the_run_form_is_none() {
        for raw in [
            "",
            "wakeshot",
            "wakeshot status",
            "wakeshot run",
            "wakeshot run --",
            "wakeshot run --  \t ",
            "wakeshot run npm test",
            "wakeshot --dry-run",
            "wakeshot status run -- x",
            "wakeshot run --dry-run -- x",
            "wakeshot run --- x",
        ] {
            assert_eq!(tail(raw), None, "{raw:?}");
        }
    }

    #[test]
    fn microsofts_examples_split_the_documented_way() {
        // "Parsing C command-line arguments" (Microsoft Learn, updated 2021-12-09): the input column and argv[1..].
        for (input, want) in [
            (r#""a b c" d e"#, &["a b c", "d", "e"][..]),
            (r#""ab\"c" "\\" d"#, &[r#"ab"c"#, r"\", "d"]),
            (r#"a\\\b d"e f"g h"#, &[r"a\\\b", "de fg", "h"]),
            (r#"a\\\"b c d"#, &[r#"a\"b"#, "c", "d"]),
            (r#"a\\\\"b c" d e"#, &[r"a\\b c", "d", "e"]),
            (r#"a"b"" c d"#, &[r#"ab" c d"#]),
        ] {
            assert_eq!(split_all(input), want, "{input}");
        }
    }

    #[test]
    fn split_program_gives_the_program_and_the_raw_rest() {
        let split = |tail: &str| {
            let (program, rest) = split_program(OsStr::new(tail)).unwrap();
            (program.into_string().unwrap(), rest.into_string().unwrap())
        };
        assert_eq!(
            split(r#""C:\Program Files\x.exe" a "b  c""#),
            (r"C:\Program Files\x.exe".into(), r#"a "b  c""#.into())
        );
        assert_eq!(split("npm"), ("npm".into(), String::new()));
        assert_eq!(split("npm \t  test  "), ("npm".into(), "test  ".into()));
        assert_eq!(split(r#"""  x"#), (String::new(), "x".into()));
        assert_eq!(split_program(OsStr::new("  ")), None);
    }

    fn split_all(text: &str) -> Vec<String> {
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut out = Vec::new();
        let mut at = 0;
        while let Some((value, end)) = next_arg(&units, at) {
            out.push(String::from_utf16(&value).unwrap());
            at = end;
        }
        out
    }

    /// One argument quoted the MSVC way: std's own form (`\"` inside), or with `""` inside quotes.
    fn quote(arg: &str, force: bool, doubled: bool) -> String {
        let quoted = force || doubled || arg.is_empty() || arg.contains([' ', '\t']);
        let mut out = String::new();
        if quoted {
            out.push('"');
        }
        let mut slashes = 0;
        for c in arg.chars() {
            match c {
                '\\' => slashes += 1,
                '"' if doubled => {
                    out.extend(std::iter::repeat_n('\\', slashes));
                    out.push('"');
                    slashes = 0;
                }
                '"' => {
                    out.extend(std::iter::repeat_n('\\', slashes + 1));
                    slashes = 0;
                }
                _ => slashes = 0,
            }
            out.push(c);
        }
        if quoted {
            out.extend(std::iter::repeat_n('\\', slashes));
            out.push('"');
        }
        out
    }

    fn an_arg() -> impl Strategy<Value = String> {
        prop_oneof![
            4 => proptest::collection::vec(
                prop::sample::select(vec!['a', 'b', ' ', '\t', '"', '\\', '-', '\u{561}', '\u{1F600}']),
                0..8,
            )
            .prop_map(|chars| chars.into_iter().collect()),
            1 => prop::sample::select(vec!["--", "run", "--dry-run", "-- x", ""]).prop_map(String::from),
        ]
    }

    fn a_blank() -> impl Strategy<Value = String> {
        proptest::collection::vec(prop::sample::select(vec![' ', '\t']), 1..3)
            .prop_map(|chars| chars.into_iter().collect())
    }

    proptest! {
        #[test]
        fn run_tail_returns_exactly_the_quoted_child_part(
            program in prop::sample::select(vec![
                "wakeshot",
                "wakeshot.exe",
                r#""C:\Program Files\Wakeshot\wakeshot.exe""#,
                r"C:\Users\a\wakeshot.exe",
                r#"C:\"a b"\wakeshot.exe"#,
            ]),
            dry_run in any::<bool>(),
            run_word in prop::sample::select(vec!["run", "RUN", r#""run""#]),
            dashes in prop::sample::select(vec!["--", r#""--""#]),
            child in proptest::collection::vec((an_arg(), any::<bool>(), any::<bool>(), a_blank()), 1..6),
            blank in a_blank(),
        ) {
            let mut child_part = String::new();
            for (i, (arg, force, doubled, sep)) in child.iter().enumerate() {
                if i > 0 {
                    child_part.push_str(sep);
                }
                child_part.push_str(&quote(arg, *force, *doubled));
            }
            let flag = if dry_run { format!("--dry-run{blank}") } else { String::new() };
            let raw = format!("{program}{blank}{flag}{run_word}{blank}{dashes}{blank}{child_part}");

            prop_assert_eq!(tail(&raw), Some(child_part.clone()), "{}", raw);
            let want: Vec<String> = child.iter().map(|(arg, ..)| arg.clone()).collect();
            prop_assert_eq!(split_all(&child_part), want.clone());

            let mut argv = vec![OsString::from(RUN), OsString::from("--")];
            if dry_run {
                argv.insert(0, OsString::from(DRY_RUN));
            }
            argv.extend(want.iter().map(OsString::from));
            let command = match want[0].is_empty() {
                true => Command::Bad("the command after '--' is empty".into()),
                false => Command::Run,
            };
            prop_assert_eq!(parse(&argv), (Global { dry_run }, command));
        }
    }
}
