//! How the tray exe was started (architecture section 4, "Processes" and "Tray startup").

use std::ffi::OsString;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Tray,
    Guard,
    Restore,
    DryRun,
}

/// `None` for a command line no mode accepts.
pub fn parse(args: &[OsString]) -> Option<Mode> {
    let first = args.first().map(|a| a.to_str());
    match first {
        None => Some(Mode::Tray),
        // `guard::run` parses the rest (pid, start time, stop event).
        Some(Some("--guard")) => Some(Mode::Guard),
        Some(Some("--restore")) if args.len() == 1 => Some(Mode::Restore),
        Some(Some("--dry-run")) if args.len() == 1 => Some(Mode::DryRun),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_each_mode() {
        assert_eq!(parse(&args(&[])), Some(Mode::Tray));
        assert_eq!(
            parse(&args(&["--guard", "1", "2", "ev"])),
            Some(Mode::Guard)
        );
        assert_eq!(parse(&args(&["--restore"])), Some(Mode::Restore));
        assert_eq!(parse(&args(&["--dry-run"])), Some(Mode::DryRun));
        assert_eq!(parse(&args(&["--restore", "x"])), None);
        assert_eq!(parse(&args(&["--nope"])), None);
    }
}
