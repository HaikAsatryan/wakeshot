//! The step file. The whole file is parsed before the child starts, so a typo fails at once instead of halfway
//! through a live agent session that cost a real turn.

use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum EnvChange {
    Set(String, String),
    Unset(String),
    /// The value is the trimmed stdout of a program. It goes only into the child's environment block: never into a
    /// log, a file or the done file, so a token can pass without being seen.
    FromProgram(String, Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub cols: u16,
    pub rows: u16,
    pub timeout: Duration,
    pub cwd: Option<String>,
    pub env: Vec<EnvChange>,
    pub type_delay: Duration,
    pub raw: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cols: 120,
            rows: 40,
            timeout: Duration::from_secs(600),
            cwd: None,
            env: Vec::new(),
            // A person types far slower; Codex reads a fast burst of keys as a paste, where Enter becomes a newline.
            type_delay: Duration::from_millis(15),
            raw: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Wait {
        limit: Duration,
        any: Vec<String>,
    },
    WaitGone {
        limit: Duration,
        text: String,
    },
    /// Waits until `text` shows more often than when the step began: the end marker of a new turn, when old turns
    /// with the same marker are still on screen.
    WaitNew {
        limit: Duration,
        text: String,
    },
    Quiet {
        limit: Duration,
        still: Duration,
    },
    Type(String),
    Paste(String),
    Send(Vec<u8>),
    Sleep(Duration),
    Snap(String),
    Kill(String),
    KillTree,
    WaitExit(Duration),
    Pids,
    Serve,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub line: usize,
    pub source: String,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub settings: Settings,
    pub command_line: String,
    pub steps: Vec<Step>,
}

pub fn parse_script(text: &str) -> Result<Script, String> {
    let mut settings = Settings::default();
    let mut command_line = None;
    let mut steps = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (word, rest) = split_word(trimmed);
        let at = |e: String| format!("line {line}: {e}");
        if command_line.is_none() {
            if word == "run" {
                if rest.is_empty() {
                    return Err(at("run needs a command line".into()));
                }
                command_line = Some(rest.to_string());
            } else {
                apply_setting(&mut settings, word, rest).map_err(at)?;
            }
        } else {
            steps.push(Step {
                line,
                source: trimmed.to_string(),
                action: parse_action(word, rest).map_err(at)?,
            });
        }
    }
    let command_line = command_line.ok_or("no run line")?;
    Ok(Script {
        settings,
        command_line,
        steps,
    })
}

/// A file dropped into the inbox while the driver serves: steps only.
pub fn parse_steps(text: &str) -> Result<Vec<Step>, String> {
    let mut steps = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (word, rest) = split_word(trimmed);
        let action = parse_action(word, rest).map_err(|e| format!("line {}: {e}", index + 1))?;
        steps.push(Step {
            line: index + 1,
            source: trimmed.to_string(),
            action,
        });
    }
    Ok(steps)
}

fn apply_setting(settings: &mut Settings, word: &str, rest: &str) -> Result<(), String> {
    let args = tokens(rest)?;
    match word {
        "size" => {
            let [cols, rows] = exact::<2>(&args, "size <cols> <rows>")?;
            settings.cols = number(&cols)?;
            settings.rows = number(&rows)?;
            if settings.cols < 20 || settings.rows < 5 {
                return Err("size is too small".into());
            }
        }
        "timeout" => {
            let [secs] = exact::<1>(&args, "timeout <secs>")?;
            settings.timeout = seconds(&secs)?;
        }
        "cwd" => {
            let [dir] = exact::<1>(&args, "cwd <folder>")?;
            settings.cwd = Some(dir);
        }
        "env" => {
            let [name, value] = exact::<2>(&args, "env <name> <value>")?;
            settings.env.push(EnvChange::Set(env_name(name)?, value));
        }
        "unset" => {
            let [name] = exact::<1>(&args, "unset <name>")?;
            settings.env.push(EnvChange::Unset(env_name(name)?));
        }
        "env-from" => {
            if args.len() < 2 {
                return Err("env-from <name> <program> [args]".into());
            }
            let name = env_name(args[0].clone())?;
            settings
                .env
                .push(EnvChange::FromProgram(name, args[1..].to_vec()));
        }
        "type-delay" => {
            let [ms] = exact::<1>(&args, "type-delay <ms>")?;
            settings.type_delay = Duration::from_millis(number(&ms)?);
        }
        "raw" => {
            let [flag] = exact::<1>(&args, "raw on|off")?;
            settings.raw = match flag.as_str() {
                "on" => true,
                "off" => false,
                _ => return Err("raw on|off".into()),
            };
        }
        _ => return Err(format!("unknown setting `{word}` before run")),
    }
    Ok(())
}

fn parse_action(word: &str, rest: &str) -> Result<Action, String> {
    let action = match word {
        "wait" => {
            let args = tokens(rest)?;
            if args.len() < 2 {
                return Err("wait <secs> <text> [<text> ...]".into());
            }
            Action::Wait {
                limit: seconds(&args[0])?,
                any: args[1..].to_vec(),
            }
        }
        "wait-gone" => {
            let [secs, text] = exact::<2>(&tokens(rest)?, "wait-gone <secs> <text>")?;
            Action::WaitGone {
                limit: seconds(&secs)?,
                text,
            }
        }
        "wait-new" => {
            let [secs, text] = exact::<2>(&tokens(rest)?, "wait-new <secs> <text>")?;
            Action::WaitNew {
                limit: seconds(&secs)?,
                text,
            }
        }
        "quiet" => {
            let [secs, ms] = exact::<2>(&tokens(rest)?, "quiet <secs> <still ms>")?;
            Action::Quiet {
                limit: seconds(&secs)?,
                still: Duration::from_millis(number(&ms)?),
            }
        }
        "type" => Action::Type(text_arg(rest)?),
        "paste" => Action::Paste(text_arg(rest)?),
        "send" => {
            let [seq] = exact::<1>(&tokens(rest)?, "send \"<escaped bytes>\"")?;
            Action::Send(seq.into_bytes())
        }
        "key" => {
            let args = tokens(rest)?;
            if args.is_empty() || args.len() > 2 {
                return Err("key <name> [count]".into());
            }
            let one = key_bytes(&args[0])?;
            let count: usize = match args.get(1) {
                Some(n) => number(n)?,
                None => 1,
            };
            Action::Send(one.repeat(count))
        }
        "enter" | "esc" | "tab" | "up" | "down" => {
            if !rest.is_empty() {
                return Err(format!("{word} takes no arguments"));
            }
            Action::Send(key_bytes(word)?)
        }
        "sleep" => {
            let [secs] = exact::<1>(&tokens(rest)?, "sleep <secs>")?;
            Action::Sleep(seconds(&secs)?)
        }
        "snap" => Action::Snap(rest.to_string()),
        "kill" => {
            let [image] = exact::<1>(&tokens(rest)?, "kill <pid or image.exe>")?;
            Action::Kill(image)
        }
        "kill-tree" => Action::KillTree,
        "wait-exit" => {
            let [secs] = exact::<1>(&tokens(rest)?, "wait-exit <secs>")?;
            Action::WaitExit(seconds(&secs)?)
        }
        "pids" => Action::Pids,
        "serve" => Action::Serve,
        "end" => Action::End,
        "run" => return Err("run may appear only once".into()),
        _ => return Err(format!("unknown step `{word}`")),
    };
    Ok(action)
}

/// Key names map to what Windows Terminal sends for that key, without win32-input-mode; the pseudo console turns
/// them back into key events.
pub fn key_bytes(name: &str) -> Result<Vec<u8>, String> {
    let lower = name.to_ascii_lowercase();
    let seq: &[u8] = match lower.as_str() {
        "enter" => b"\r",
        // The kitty keyboard form of Enter. Claude Code under a pseudo console took Enter only this way
        // (`spikes/claude-watch/ccdrive.py`).
        "kenter" => b"\x1b[13u",
        "esc" => b"\x1b",
        "kesc" => b"\x1b[27u",
        "tab" => b"\t",
        "shift-tab" => b"\x1b[Z",
        "up" => b"\x1b[A",
        "down" => b"\x1b[B",
        "right" => b"\x1b[C",
        "left" => b"\x1b[D",
        "home" => b"\x1b[H",
        "end" => b"\x1b[F",
        "pgup" => b"\x1b[5~",
        "pgdn" => b"\x1b[6~",
        "delete" => b"\x1b[3~",
        "backspace" => b"\x7f",
        "space" => b" ",
        _ => {
            if let Some(letter) = lower.strip_prefix("ctrl-")
                && let [c @ b'a'..=b'z'] = letter.as_bytes()
            {
                return Ok(vec![c & 0x1f]);
            }
            return Err(format!("unknown key `{name}`"));
        }
    };
    Ok(seq.to_vec())
}

fn split_word(line: &str) -> (&str, &str) {
    match line.find(char::is_whitespace) {
        Some(i) => (&line[..i], line[i..].trim()),
        None => (line, ""),
    }
}

/// `type` and `paste` take the rest of the line as is, or one quoted string when it needs escapes or edge spaces.
fn text_arg(rest: &str) -> Result<String, String> {
    if rest.starts_with('"') {
        let args = tokens(rest)?;
        if let [one] = args.as_slice() {
            return Ok(one.clone());
        }
    }
    if rest.is_empty() {
        return Err("no text".into());
    }
    Ok(rest.to_string())
}

/// Splits on whitespace; double quotes group, with `\"`, `\\`, `\n`, `\r`, `\t`, `\e` and `\xNN` inside them.
pub fn tokens(text: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        let Some(&first) = chars.peek() else {
            return Ok(out);
        };
        let mut token = String::new();
        if first == '"' {
            chars.next();
            loop {
                match chars.next() {
                    None => return Err("unclosed quote".into()),
                    Some('"') => break,
                    Some('\\') => token.push(unescape(&mut chars)?),
                    Some(c) => token.push(c),
                }
            }
            if chars.peek().is_some_and(|c| !c.is_whitespace()) {
                return Err("text right after a closing quote".into());
            }
        } else {
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                token.push(c);
                chars.next();
            }
        }
        out.push(token);
    }
}

fn unescape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<char, String> {
    Ok(match chars.next() {
        Some('"') => '"',
        Some('\\') => '\\',
        Some('n') => '\n',
        Some('r') => '\r',
        Some('t') => '\t',
        Some('e') => '\x1b',
        Some('x') => {
            let hex: String = chars.by_ref().take(2).collect();
            let value = u8::from_str_radix(&hex, 16).map_err(|_| format!("bad \\x{hex}"))?;
            if value > 0x7f {
                return Err("\\x above 7f".into());
            }
            char::from(value)
        }
        other => return Err(format!("unknown escape \\{}", other.unwrap_or(' '))),
    })
}

fn exact<const N: usize>(args: &[String], usage: &str) -> Result<[String; N], String> {
    <[String; N]>::try_from(args.to_vec()).map_err(|_| usage.to_string())
}

fn number<T: std::str::FromStr>(text: &str) -> Result<T, String> {
    text.parse()
        .map_err(|_| format!("`{text}` is not a number"))
}

fn seconds(text: &str) -> Result<Duration, String> {
    let secs: f64 = number(text)?;
    Duration::try_from_secs_f64(secs).map_err(|_| format!("`{text}` is not a time in seconds"))
}

fn env_name(name: String) -> Result<String, String> {
    if name.is_empty() || name.contains('=') {
        return Err(format!("bad variable name `{name}`"));
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_script_parses() {
        let text = "# test\nsize 100 30\ntimeout 90\ncwd \"C:\\\\a b\"\nenv TERM xterm-256color\nunset CLAUDECODE\n\
                    env-from TOKEN gh auth token\ntype-delay 40\nrun cmd /c \"set /p X=Name? \"\n\
                    wait 10 Name?\ntype hello there\nenter\nwait 5 \"Allow once\" Deny\nkey down 2\nsnap after\n\
                    quiet 5 500\nkill node.exe\nwait-exit 10\n";
        let script = parse_script(text).unwrap();
        assert_eq!(script.settings.cols, 100);
        assert_eq!(script.settings.rows, 30);
        assert_eq!(script.settings.timeout, Duration::from_secs(90));
        assert_eq!(script.settings.cwd.as_deref(), Some("C:\\a b"));
        assert_eq!(script.settings.type_delay, Duration::from_millis(40));
        assert_eq!(
            script.settings.env,
            vec![
                EnvChange::Set("TERM".into(), "xterm-256color".into()),
                EnvChange::Unset("CLAUDECODE".into()),
                EnvChange::FromProgram(
                    "TOKEN".into(),
                    vec!["gh".into(), "auth".into(), "token".into()]
                ),
            ]
        );
        assert_eq!(script.command_line, "cmd /c \"set /p X=Name? \"");
        let actions: Vec<_> = script.steps.iter().map(|s| s.action.clone()).collect();
        assert_eq!(
            actions,
            vec![
                Action::Wait {
                    limit: Duration::from_secs(10),
                    any: vec!["Name?".into()]
                },
                Action::Type("hello there".into()),
                Action::Send(b"\r".to_vec()),
                Action::Wait {
                    limit: Duration::from_secs(5),
                    any: vec!["Allow once".into(), "Deny".into()]
                },
                Action::Send(b"\x1b[B\x1b[B".to_vec()),
                Action::Snap("after".into()),
                Action::Quiet {
                    limit: Duration::from_secs(5),
                    still: Duration::from_millis(500)
                },
                Action::Kill("node.exe".into()),
                Action::WaitExit(Duration::from_secs(10)),
            ]
        );
        assert_eq!(script.steps[0].line, 10);
    }

    #[test]
    fn errors_name_the_line() {
        assert_eq!(
            parse_script("run x\nwiat 3 a\n").unwrap_err(),
            "line 2: unknown step `wiat`"
        );
        assert_eq!(
            parse_script("size 10\nrun x\n").unwrap_err(),
            "line 1: size <cols> <rows>"
        );
        assert_eq!(
            parse_script("wait 1 a\n").unwrap_err(),
            "line 1: unknown setting `wait` before run"
        );
        assert_eq!(parse_script("timeout 5\n").unwrap_err(), "no run line");
        assert_eq!(
            parse_script("run a\nrun b\n").unwrap_err(),
            "line 2: run may appear only once"
        );
        assert!(parse_script("run a\nwait x a\n").is_err());
        assert!(parse_script("run a\nwait -1 a\n").is_err());
        assert!(parse_script("env A=B c\nrun a\n").is_err());
    }

    #[test]
    fn quotes_and_escapes() {
        assert_eq!(
            tokens(r#"a "b c" "\"q\"" "\e[13u" "\x03" """#).unwrap(),
            vec!["a", "b c", "\"q\"", "\x1b[13u", "\x03", ""]
        );
        assert!(tokens("\"open").is_err());
        assert!(tokens("\"a\"b").is_err());
        assert!(tokens(r#""\xff""#).is_err());
        assert!(tokens(r#""\q""#).is_err());
    }

    #[test]
    fn type_keeps_the_line_or_one_quoted_string() {
        let steps = parse_steps("type  say \"hi\" now\ntype \" padded \"\npaste a\\nb\n").unwrap();
        assert_eq!(steps[0].action, Action::Type("say \"hi\" now".into()));
        assert_eq!(steps[1].action, Action::Type(" padded ".into()));
        assert_eq!(steps[2].action, Action::Paste("a\\nb".into()));
        assert!(parse_steps("type\n").is_err());
    }

    #[test]
    fn keys() {
        assert_eq!(key_bytes("Enter").unwrap(), b"\r");
        assert_eq!(key_bytes("esc").unwrap(), b"\x1b");
        assert_eq!(key_bytes("ctrl-c").unwrap(), b"\x03");
        assert_eq!(key_bytes("ctrl-z").unwrap(), b"\x1a");
        assert_eq!(key_bytes("kenter").unwrap(), b"\x1b[13u");
        assert!(key_bytes("ctrl-1").is_err());
        assert!(key_bytes("ctrl-ab").is_err());
        assert!(key_bytes("f13").is_err());
        assert_eq!(
            parse_steps("key tab 3\n").unwrap()[0].action,
            Action::Send(b"\t\t\t".to_vec())
        );
        assert!(parse_steps("enter now\n").is_err());
    }

    #[test]
    fn wait_new_takes_one_text() {
        assert_eq!(
            parse_steps(
                "wait-new 60 \"tok/s\"
"
            )
            .unwrap()[0]
                .action,
            Action::WaitNew {
                limit: Duration::from_secs(60),
                text: "tok/s".into()
            }
        );
        assert!(
            parse_steps(
                "wait-new 60 a b
"
            )
            .is_err()
        );
    }

    #[test]
    fn inbox_files_hold_steps_only() {
        let steps = parse_steps("# next turn\nwait 5 ok\nend\n").unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[1].action, Action::End);
        assert_eq!(
            parse_steps("size 1 2\n").unwrap_err(),
            "line 1: unknown step `size`"
        );
    }
}
