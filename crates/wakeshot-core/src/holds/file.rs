//! Hold files: parsing them into `HoldFacts`, and the RFC 3339 form of `expires` (architecture section 6).
//!
//! Owner: sources track (`p4-hold-files`; fixes by `p4-live-claude`). The two signatures below are contract: the
//! core's Keep awake (Phase 2) formats `expires` with `format_expires`, so the core writes no date code of its own.
//!
//! Pure: the watch thread reads the bytes and asks for the process lookup; this module only decides. The user doc
//! for the format is `docs/hold-files.md`.

use crate::files::{self, HoldFile, Read};
use crate::holds::{HoldFacts, HoldId, HoldKind};
use crate::process::{Alive, FileTime, Lookup};

/// The longest `reason` kept, in characters: the tooltip has little room, and a script may write a whole command line.
pub const REASON_CHARS: usize = 60;

/// `9999-12-31T23:59:59Z` in seconds since 1970: RFC 3339 has four-digit years only.
const MAX_SECS: i64 = 253_402_300_799;

/// The hold kind a file name gives, or `None` for a name that is not a hold file (never read).
///
/// Case is ignored, because NTFS names are: `KEEP-AWAKE.JSON` is the same file as `keep-awake.json`, and a `*.json`
/// listing returns it.
pub fn kind_of(name: &str) -> Option<HoldKind> {
    let stem = strip_suffix_ignore_case(name, ".json")?;
    if stem.eq_ignore_ascii_case("keep-awake") {
        return Some(HoldKind::KeepAwake);
    }
    let run_pid = strip_prefix_ignore_case(stem, "run-");
    if run_pid
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
    {
        return Some(HoldKind::Run);
    }
    Some(HoldKind::Script)
}

/// One hold file as it counts: what `parse` made of the name and the bytes.
///
/// `file` is the file as written, for `hold_alive` (`pid`, `procStart`); the fields beside it are what counts, so
/// read `reason` and `screen` here, never from `file`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hold {
    pub kind: HoldKind,
    pub file: HoldFile,
    pub expires_wall_ms: Option<i64>,
    /// Control characters made spaces, trimmed, cut to [`REASON_CHARS`]; `None` when nothing is left.
    pub reason: Option<String>,
    /// False outside `keep-awake.json`, whatever the file says.
    pub screen: bool,
}

/// What `parse` found. `Bad` carries a fixed reason for the log, never file content, since a script may have
/// written anything there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parsed {
    Ok(Hold),
    Newer(u32),
    Bad(&'static str),
}

/// A hold file's name and bytes to a `Hold`, by the version rule of [`files::read`] (first JSON value only, a byte
/// order mark skipped).
pub fn parse(name: &str, bytes: &[u8]) -> Parsed {
    let Some(kind) = kind_of(name) else {
        return Parsed::Bad("the name does not end in .json");
    };
    let file: HoldFile = match files::read(bytes) {
        Read::Value(file) => file,
        Read::Newer(version) => return Parsed::Newer(version),
        Read::Bad => return Parsed::Bad("not a hold file: JSON with version and pid"),
    };
    // Pid 0 is the System Idle Process: it never writes a file and never ends, so such a hold could never end.
    if file.pid == 0 {
        return Parsed::Bad("pid 0 is not a process");
    }
    // Track note 10: a hold whose end cannot be read is no hold, rather than a hold with no end.
    let expires_wall_ms = match file.expires.as_deref().map(parse_expires) {
        None => None,
        Some(Some(wall_ms)) => Some(wall_ms),
        Some(None) => return Parsed::Bad("expires is not an RFC 3339 time with Z or an offset"),
    };
    Parsed::Ok(Hold {
        kind,
        expires_wall_ms,
        reason: file.reason.as_deref().and_then(cut_reason),
        screen: kind == HoldKind::KeepAwake && file.screen,
        file,
    })
}

/// `expires` text (RFC 3339) to wall ms (UTC); `None` when it does not parse.
///
/// Takes RFC 3339 section 5.6 `date-time`: `Z` or a `+hh:mm` / `-hh:mm` offset, an optional fraction kept to the
/// millisecond, years 1970 to 9999, `T` and `Z` in either case (section 5.6, note). A leap second (`:60`) reads as
/// `:59`. Nothing else: no space for `T`, no missing zone, no white space around it.
pub fn parse_expires(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let year = number(&b[0..4])?;
    let month = number(&b[5..7])?;
    let day = number(&b[8..10])?;
    let hour = number(&b[11..13])?;
    let minute = number(&b[14..16])?;
    let second = number(&b[17..19])?;
    if !(1970..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let mut rest = &b[19..];
    let mut millis = 0;
    if let Some(fraction) = rest.strip_prefix(b".") {
        let len = fraction.iter().take_while(|b| b.is_ascii_digit()).count();
        if len == 0 {
            return None;
        }
        // Cut, never rounded: rounding could carry into the next second, or past 9999.
        let kept = &fraction[..len.min(3)];
        millis = number(kept)? * 10_i64.pow(3 - kept.len() as u32);
        rest = &fraction[len..];
    }
    let offset_ms = match rest {
        [b'Z' | b'z'] => 0,
        [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2] => {
            let (hours, minutes) = (number(&[*h1, *h2])?, number(&[*m1, *m2])?);
            if hours > 23 || minutes > 59 {
                return None;
            }
            let ms = (hours * 60 + minutes) * 60_000;
            if *sign == b'+' { ms } else { -ms }
        }
        _ => return None,
    };

    // A leap second is off by one second at most, which no hold notices.
    let second = second.min(59);
    let days = days_from_civil(year, month, day);
    Some(((days * 24 + hour) * 60 + minute) * 60_000 + second * 1000 + millis - offset_ms)
}

/// Wall ms to `expires` text (RFC 3339, UTC), the form `parse_expires` reads back.
///
/// Whole seconds (the milliseconds are dropped), `Z`, such as `2026-11-03T18:00:00Z`. A time before 1970 or after
/// 9999 is clamped to that range, so the text always reads back.
pub fn format_expires(wall_ms: i64) -> String {
    let secs = wall_ms.div_euclid(1000).clamp(0, MAX_SECS);
    let (days, of_day) = (secs / 86_400, secs % 86_400);
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second) = (of_day / 3600, of_day / 60 % 60, of_day % 60);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Whether a hold file's process still runs (architecture section 6), from one lookup of `file.pid`.
///
/// With `procStart`, only that exact process counts. Without it, a process that started after the file's last write
/// cannot have written it (a reused pid), so it counts only when it started at or before `written`. The rule keeps
/// no state, so it gives the same answer after a tray restart.
pub fn hold_alive(file: &HoldFile, running: Lookup, written: FileTime) -> Alive {
    match running {
        Lookup::Unknown => Alive::Unknown,
        Lookup::NotRunning => Alive::Gone,
        Lookup::Running(start) => match file.proc_start {
            Some(proc_start) if proc_start == start => Alive::Yes,
            None if start <= written => Alive::Yes,
            _ => Alive::Gone,
        },
    }
}

/// The snapshot entry for `Event::Holds`.
pub fn facts(id: HoldId, hold: &Hold, alive: Alive) -> HoldFacts {
    HoldFacts {
        id,
        kind: hold.kind,
        alive,
        expires_wall_ms: hold.expires_wall_ms,
        reason: hold.reason.clone(),
        screen: hold.screen,
    }
}

fn cut_reason(text: &str) -> Option<String> {
    // A line break or other control character would split the log line and the tooltip.
    let cut: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .skip_while(|c| c.is_whitespace())
        .take(REASON_CHARS)
        .collect();
    let cut = cut.trim_end();
    (!cut.is_empty()).then(|| cut.to_owned())
}

fn strip_suffix_ignore_case<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    let split = text.len().checked_sub(suffix.len())?;
    let (head, tail) = (text.get(..split)?, text.get(split..)?);
    tail.eq_ignore_ascii_case(suffix).then_some(head)
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let (head, tail) = (text.get(..prefix.len())?, text.get(prefix.len()..)?);
    head.eq_ignore_ascii_case(prefix).then_some(tail)
}

/// ASCII digits only: `str::parse` would also take a sign.
fn number(digits: &[u8]) -> Option<i64> {
    digits.iter().try_fold(0_i64, |n, &b| {
        b.is_ascii_digit().then(|| n * 10 + i64::from(b - b'0'))
    })
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

// Days from 1970-01-01 and back, proleptic Gregorian: Howard Hinnant's `days_from_civil` and `civil_from_days`
// (public domain), with years counted from March so the leap day falls at the end.

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Architecture section 6, "Hold format".
    const DOCUMENTED: &str = r#"{ "version": 1, "pid": 4242, "procStart": "134381211391959336",
  "reason": "nightly backup", "expires": "2026-11-03T18:00:00Z", "screen": false }"#;

    /// `2026-11-03T18:00:00Z`; this and the other wall ms below come from .NET `DateTimeOffset` (docs/research.md).
    const SIX_PM: i64 = 1_793_728_800_000;

    fn ok(name: &str, text: &str) -> Hold {
        match parse(name, text.as_bytes()) {
            Parsed::Ok(hold) => hold,
            other => panic!("{name} {text:?}: {other:?}"),
        }
    }

    fn with_reason(reason: &str) -> Option<String> {
        let file = HoldFile {
            reason: Some(reason.to_owned()),
            ..HoldFile::new(1, None)
        };
        ok("job.json", &serde_json::to_string(&file).unwrap()).reason
    }

    #[test]
    fn names_give_kinds() {
        for (name, kind) in [
            ("keep-awake.json", Some(HoldKind::KeepAwake)),
            ("KEEP-AWAKE.JSON", Some(HoldKind::KeepAwake)),
            ("run-4242.json", Some(HoldKind::Run)),
            ("run-0.json", Some(HoldKind::Run)),
            ("Run-12.Json", Some(HoldKind::Run)),
            ("nightly-backup.json", Some(HoldKind::Script)),
            ("run-.json", Some(HoldKind::Script)),
            ("run-12a.json", Some(HoldKind::Script)),
            ("run--12.json", Some(HoldKind::Script)),
            ("run-+12.json", Some(HoldKind::Script)),
            ("run-\u{661}\u{662}.json", Some(HoldKind::Script)),
            ("keep-awake-2.json", Some(HoldKind::Script)),
            ("my keep-awake.json", Some(HoldKind::Script)),
            ("\u{561}\u{562}.json", Some(HoldKind::Script)),
            (".json", Some(HoldKind::Script)),
            ("keep-awake.json.4242.0.tmp", None),
            ("nightly-backup.tmp", None),
            ("backup.jsonx", None),
            ("backup.json ", None),
            ("json", None),
            ("", None),
            ("\u{561}json", None),
        ] {
            assert_eq!(kind_of(name), kind, "{name:?}");
        }
    }

    #[test]
    fn reads_the_documented_form() {
        assert_eq!(
            ok("nightly-backup.json", DOCUMENTED),
            Hold {
                kind: HoldKind::Script,
                file: HoldFile {
                    reason: Some("nightly backup".into()),
                    expires: Some("2026-11-03T18:00:00Z".into()),
                    ..HoldFile::new(4242, Some(FileTime(134_381_211_391_959_336)))
                },
                expires_wall_ms: Some(SIX_PM),
                reason: Some("nightly backup".into()),
                screen: false,
            }
        );
    }

    /// The JSON in the user doc is the architecture's form, so the two cannot drift apart.
    #[test]
    fn the_user_doc_json_is_the_documented_form() {
        let doc = include_str!("../../../../docs/hold-files.md");
        let blocks: Vec<&str> = doc
            .split("```json")
            .skip(1)
            .filter_map(|rest| rest.split("```").next())
            .collect();
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            ok("nightly-backup.json", blocks[0]),
            ok("nightly-backup.json", DOCUMENTED)
        );
    }

    /// Byte for byte what the user doc's examples wrote, run in a temp folder (2026-11-03, docs/research.md):
    /// PowerShell 7.6.9, Windows PowerShell 5.1 and cmd.
    #[test]
    fn files_the_doc_examples_wrote_parse() {
        let pwsh = br#"{"version":1,"pid":32416,"procStart":"134381955000279373","reason":"nightly backup","expires":"2026-11-03T20:05:00.2920348Z"}"#;
        let windows_powershell = br#"{"version":1,"pid":7056,"procStart":"134381955003940356","reason":"nightly backup","expires":"2026-11-03T20:05:01.1414071Z"}"#;
        let cmd = b"{\"version\": 1, \"pid\": 8164, \"reason\": \"nightly backup\"}\r\n";
        for (bytes, pid, proc_start, expires_wall_ms) in [
            (
                &pwsh[..],
                32416,
                Some(134_381_955_000_279_373),
                Some(1_793_736_300_292),
            ),
            (
                &windows_powershell[..],
                7056,
                Some(134_381_955_003_940_356),
                Some(1_793_736_301_141),
            ),
            (&cmd[..], 8164, None, None),
        ] {
            let Parsed::Ok(hold) = parse("nightly-backup.json", bytes) else {
                panic!("{}", String::from_utf8_lossy(bytes));
            };
            assert_eq!(hold.kind, HoldKind::Script);
            assert_eq!(hold.file.pid, pid);
            assert_eq!(hold.file.proc_start, proc_start.map(FileTime));
            assert_eq!(hold.expires_wall_ms, expires_wall_ms);
            assert_eq!(hold.reason.as_deref(), Some("nightly backup"));
        }
    }

    #[test]
    fn reads_a_minimal_file() {
        assert_eq!(
            ok("run-7.json", r#"{"version":1,"pid":7}"#),
            Hold {
                kind: HoldKind::Run,
                file: HoldFile::new(7, None),
                expires_wall_ms: None,
                reason: None,
                screen: false,
            }
        );
    }

    #[test]
    fn a_newer_version_is_newer_whatever_its_fields() {
        let text = r#"{ "version": 2, "pid": "4242", "expires": "tomorrow" }"#;
        assert_eq!(parse("job.json", text.as_bytes()), Parsed::Newer(2));
    }

    #[test]
    fn what_is_not_a_hold_file_is_bad() {
        for text in [
            "",
            "not json",
            "[1]",
            "{ \"version\": 1, \"pid\": 4242",
            r#"{ "version": 1 }"#,
            r#"{ "version": 1, "pid": -1 }"#,
            r#"{ "version": 1, "pid": "4242" }"#,
            r#"{ "version": 1, "pid": 4242, "procStart": 134381211391959336 }"#,
            r#"{ "version": 1, "pid": 4242, "reason": 5 }"#,
        ] {
            assert!(
                matches!(parse("job.json", text.as_bytes()), Parsed::Bad(_)),
                "{text:?}"
            );
        }
        let pid_zero = r#"{ "version": 1, "pid": 0 }"#;
        assert_eq!(
            parse("job.json", pid_zero.as_bytes()),
            Parsed::Bad("pid 0 is not a process")
        );
        assert_eq!(
            parse("job.txt", DOCUMENTED.as_bytes()),
            Parsed::Bad("the name does not end in .json")
        );
    }

    /// Only the first JSON value counts, as for Claude's files; a byte order mark is no reason to drop a hold.
    #[test]
    fn bytes_after_the_first_value_are_ignored() {
        let want = ok("job.json", DOCUMENTED);
        for text in [
            format!("{DOCUMENTED}\r\n"),
            format!("{DOCUMENTED} {{ \"version\": 9 }} trailing"),
            format!("{DOCUMENTED}garbage"),
            format!("\u{feff}{DOCUMENTED}"),
        ] {
            assert_eq!(ok("job.json", &text), want, "{text:?}");
        }
    }

    #[test]
    fn a_bad_expires_makes_the_file_bad() {
        for expires in [
            "2026-11-03T18:00:00",
            "2026-11-03T24:00:00Z",
            "2026-13-03T18:00:00Z",
            "2025-02-29T18:00:00Z",
            "",
        ] {
            let file = HoldFile {
                expires: Some(expires.into()),
                ..HoldFile::new(4242, None)
            };
            let text = serde_json::to_string(&file).unwrap();
            assert_eq!(
                parse("job.json", text.as_bytes()),
                Parsed::Bad("expires is not an RFC 3339 time with Z or an offset"),
                "{expires:?}"
            );
        }
    }

    #[test]
    fn expires_forms_that_parse() {
        for (text, wall_ms) in [
            ("2026-11-03T18:00:00Z", SIX_PM),
            ("2026-11-03t18:00:00z", SIX_PM),
            ("2026-11-03T22:00:00+04:00", SIX_PM),
            ("2026-11-03T13:30:00-04:30", SIX_PM),
            ("2026-11-03T18:00:00-00:00", SIX_PM),
            ("2026-11-03T18:00:00.1Z", SIX_PM + 100),
            ("2026-11-03T18:00:00.12Z", SIX_PM + 120),
            ("2026-11-03T18:00:00.123Z", SIX_PM + 123),
            ("2026-11-03T18:00:00.9999Z", SIX_PM + 999),
            ("2026-11-03T18:00:00.1234567-05:30", 1_793_748_600_123),
            ("1970-01-01T00:00:00Z", 0),
            ("1970-01-01T00:00:00.001Z", 1),
            ("1970-01-01T00:00:00+01:00", -3_600_000),
            ("2000-02-29T12:34:56.789+04:00", 951_813_296_789),
            ("2024-02-29T00:00:00Z", 1_709_164_800_000),
            ("2100-03-01T00:00:00Z", 4_107_542_400_000),
            ("9999-12-31T23:59:59Z", 253_402_300_799_000),
            ("2016-12-31T23:59:60Z", 1_483_228_799_000),
        ] {
            assert_eq!(parse_expires(text), Some(wall_ms), "{text}");
        }
    }

    #[test]
    fn expires_forms_that_do_not_parse() {
        for text in [
            "2026-11-03T18:00:00",
            "2026-11-03T24:00:00Z",
            "2026-13-03T18:00:00Z",
            "2026-00-03T18:00:00Z",
            "2025-02-29T18:00:00Z",
            "2100-02-29T18:00:00Z",
            "2026-04-31T18:00:00Z",
            "2026-11-00T18:00:00Z",
            "2026-11-03T18:60:00Z",
            "2026-11-03T18:00:61Z",
            "2026-11-03 18:00:00Z",
            "2026-11-03T18:00Z",
            "2026-11-03T18:00:00.Z",
            "2026-11-03T18:00:00.12a3Z",
            "2026-11-03T18:00:00UTC",
            "2026-11-03T18:00:00+0400",
            "2026-11-03T18:00:00+04",
            "2026-11-03T18:00:00+04:00:00",
            "2026-11-03T18:00:00+24:00",
            "2026-11-03T18:00:00+04:60",
            "2026-11-03T18:00:00Z ",
            " 2026-11-03T18:00:00Z",
            "2026-1-03T18:00:00Z",
            "+2026-11-03T18:00:00Z",
            "1969-12-31T23:59:59Z",
            "10000-01-01T00:00:00Z",
            "2026-11-03T18:00:00\u{ff3a}",
            "\u{ff12}026-11-03T18:00:00Z",
            "2026-11-03",
            "",
        ] {
            assert_eq!(parse_expires(text), None, "{text:?}");
        }
    }

    #[test]
    fn expires_formats_in_utc_whole_seconds() {
        for (wall_ms, text) in [
            (0, "1970-01-01T00:00:00Z"),
            (SIX_PM, "2026-11-03T18:00:00Z"),
            (SIX_PM + 999, "2026-11-03T18:00:00Z"),
            (951_813_296_789, "2000-02-29T08:34:56Z"),
            (253_402_300_799_000, "9999-12-31T23:59:59Z"),
            (-1, "1970-01-01T00:00:00Z"),
            (i64::MIN, "1970-01-01T00:00:00Z"),
            (i64::MAX, "9999-12-31T23:59:59Z"),
        ] {
            assert_eq!(format_expires(wall_ms), text, "{wall_ms}");
        }
    }

    proptest! {
        #[test]
        fn whole_seconds_round_trip(secs in 0..=MAX_SECS) {
            let wall_ms = secs * 1000;
            prop_assert_eq!(parse_expires(&format_expires(wall_ms)), Some(wall_ms));
        }

        #[test]
        fn any_wall_ms_formats_to_text_that_reads_back(wall_ms in any::<i64>()) {
            let want = wall_ms.div_euclid(1000).clamp(0, MAX_SECS) * 1000;
            prop_assert_eq!(parse_expires(&format_expires(wall_ms)), Some(want));
        }

        /// The same instant written with an offset: local time `t + offset` followed by `+hh:mm` or `-hh:mm`.
        #[test]
        fn an_offset_names_the_same_instant(
            secs in 86_400..=(MAX_SECS - 86_400),
            offset_min in -1439_i64..=1439,
        ) {
            let wall_ms = secs * 1000;
            let local = format_expires(wall_ms + offset_min * 60_000);
            let sign = if offset_min < 0 { '-' } else { '+' };
            let (hours, minutes) = (offset_min.abs() / 60, offset_min.abs() % 60);
            let text = format!("{}{sign}{hours:02}:{minutes:02}", &local[..19]);
            prop_assert_eq!(parse_expires(&text), Some(wall_ms), "{}", text);
        }
    }

    #[test]
    fn reason_is_cut_to_60_characters() {
        let long = "a".repeat(61);
        assert_eq!(with_reason(&long), Some("a".repeat(60)));
        assert_eq!(with_reason(&long[..60]), Some("a".repeat(60)));
        // Two bytes (Armenian) and four bytes (an emoji) per character: the cut counts characters.
        for c in ['\u{561}', '\u{1F600}'] {
            let long: String = std::iter::repeat_n(c, 61).collect();
            let want: String = std::iter::repeat_n(c, 60).collect();
            assert_eq!(with_reason(&long), Some(want), "{c:?}");
        }
    }

    #[test]
    fn reason_is_one_trimmed_line() {
        assert_eq!(
            with_reason("line one\nline two\t\u{0}x"),
            Some("line one line two  x".into())
        );
        assert_eq!(with_reason("  padded \r\n"), Some("padded".into()));
        assert_eq!(
            with_reason(&format!("{} b", "a".repeat(59))),
            Some("a".repeat(59))
        );
        assert_eq!(with_reason(""), None);
        assert_eq!(with_reason(" \t\r\n"), None);
    }

    #[test]
    fn screen_counts_only_in_keep_awake_json() {
        let text = r#"{ "version": 1, "pid": 4242, "screen": true }"#;
        assert!(ok("keep-awake.json", text).screen);
        for name in ["run-4242.json", "nightly-backup.json"] {
            let hold = ok(name, text);
            assert!(!hold.screen, "{name}");
            assert!(hold.file.screen, "the file stays as written");
        }
    }

    #[test]
    fn hold_alive_with_proc_start_needs_that_process() {
        let started = FileTime(134_381_211_391_959_336);
        let file = HoldFile::new(4242, Some(started));
        let written = FileTime(started.0 + 10_000_000);
        for (lookup, alive) in [
            (Lookup::Running(started), Alive::Yes),
            (Lookup::Running(FileTime(started.0 - 1)), Alive::Gone),
            (Lookup::Running(FileTime(started.0 + 1)), Alive::Gone),
            (Lookup::NotRunning, Alive::Gone),
            (Lookup::Unknown, Alive::Unknown),
        ] {
            assert_eq!(hold_alive(&file, lookup, written), alive, "{lookup:?}");
        }
    }

    #[test]
    fn hold_alive_without_proc_start_needs_a_start_by_the_last_write() {
        let file = HoldFile::new(4242, None);
        let written = FileTime(134_381_211_391_959_336);
        for (lookup, alive) in [
            (
                Lookup::Running(FileTime(written.0 - 10_000_000)),
                Alive::Yes,
            ),
            (Lookup::Running(written), Alive::Yes),
            (Lookup::Running(FileTime(written.0 + 1)), Alive::Gone),
            (Lookup::NotRunning, Alive::Gone),
            (Lookup::Unknown, Alive::Unknown),
        ] {
            assert_eq!(hold_alive(&file, lookup, written), alive, "{lookup:?}");
        }
    }

    #[test]
    fn facts_carry_what_counts() {
        let text = r#"{ "version": 1, "pid": 9120, "reason": " Keep awake ",
                        "expires": "2026-11-03T18:00:00Z", "screen": true }"#;
        let hold = ok("keep-awake.json", text);
        assert_eq!(
            facts(HoldId("keep-awake.json".into()), &hold, Alive::Yes),
            HoldFacts {
                id: HoldId("keep-awake.json".into()),
                kind: HoldKind::KeepAwake,
                alive: Alive::Yes,
                expires_wall_ms: Some(SIX_PM),
                reason: Some("Keep awake".into()),
                screen: true,
            }
        );
    }
}
