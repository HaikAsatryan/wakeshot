//! Every user-facing string: menu, status line, tooltip pieces, notices (decision 1: English only, no translation
//! layer).
//!
//! Owner: tray track (`p5-text`, then `p5-haik-check`) words them; the signatures are contract, because the core's
//! `present` (Phase 2) calls them. Tests elsewhere build their expected words from these functions, never from
//! literals, so a wording change breaks only `wording_snapshot` here.
//!
//! One group string serves three places (`present`): the tooltip, the menu's status line and `state.json`, which
//! `wakeshot status` prints. So groups are plain text, and only [`status_line`], the one menu text built from them,
//! doubles `&` (the Win32 prefix rule: `&&` shows one `&`). The tooltip has two lines, "Wakeshot" and the status
//! line's words, as the Phase 0 tray spike showed them. No clock times: the core has no UTC offset (Phase 2 track
//! note 14), so a time left reads "25 min left".

use crate::holds::HoldKind;
use crate::settings::SettingKey;
use crate::view::{IconState, KeepAwake, MenuCommand, Notice};

/// The longest tooltip Windows shows: `szTip[128]` less the end mark, in UTF-16 units.
pub const TOOLTIP_MAX_UTF16: usize = 127;

/// `NOTIFYICONDATAW`: `szInfoTitle[64]` and `szInfo[256]`, each less the end mark, in UTF-16 units.
const TITLE_MAX_UTF16: usize = 63;
const BODY_MAX_UTF16: usize = 255;
/// The same page asks for at most 200 characters of English in a body, so the closed-lid report fits its list to it.
const BODY_ENGLISH_UTF16: usize = 200;

const NAME_LINE: &str = "Wakeshot\n";
const MINUTE_MS: u64 = 60_000;

/// What the status line and the tooltip say besides the hold groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub icon: IconState,
    pub stepped_back: bool,
    pub requests_only: bool,
    pub screen_on: bool,
}

/// What a notice's balloon click opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoticeClick {
    Nothing,
    OpenTaskbarSettings,
    OpenLog,
}

/// A notice as the balloon shows it. `silent`: the balloon makes no sound, because a chime plays with the notice or
/// the notice is a quiet one (`Updated`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoticeText {
    pub title: String,
    pub body: String,
    pub warning: bool,
    pub silent: bool,
    pub click: NoticeClick,
}

/// A menu item's text, with `&` before its access key: the first letter, except Keep &screen on, whose own word is
/// the one a user looks for. The words do not depend on a toggle's bool, which is the command the row sends, not
/// what it shows.
pub fn action(command: MenuCommand) -> String {
    match command {
        MenuCommand::Automatic(_) => "&Automatic".to_owned(),
        MenuCommand::KeepAwake(KeepAwake::Off) => "&Off".to_owned(),
        MenuCommand::KeepAwake(KeepAwake::Minutes(minutes)) => format!("&{}", length(minutes)),
        MenuCommand::KeepAwake(KeepAwake::UntilOff) => "&Until I turn it off".to_owned(),
        MenuCommand::KeepScreenOn(_) => "Keep &screen on".to_owned(),
        MenuCommand::Restore => "&Restore my power settings".to_owned(),
        MenuCommand::OpenLog => "&Open log".to_owned(),
        MenuCommand::Quit => "&Quit".to_owned(),
    }
}

/// The Keep awake submenu's own text.
pub fn keep_awake_menu() -> String {
    "&Keep awake".to_owned()
}

/// One hold group, such as two Claude Code sessions: the label as given, then the count where it says something.
/// An empty label (a hold file without a reason) gets a name by kind. Plain text with `&` as written, since the
/// tooltip and `state.json` show it too; [`status_line`] doubles `&` for the menu.
pub fn hold_group(kind: HoldKind, label: &str, count: u32) -> String {
    let n = u64::from(count);
    let name = |fallback: &str| {
        if label.is_empty() {
            fallback.to_owned()
        } else {
            label.to_owned()
        }
    };
    match kind {
        HoldKind::Agent => format!("{} ({})", name("an agent"), plural(n, "session")),
        HoldKind::Run if n > 1 => format!("{} ({})", name("wakeshot run"), plural(n, "command")),
        HoldKind::Run => name("wakeshot run"),
        HoldKind::Script if label.is_empty() && n == 1 => "a hold file".to_owned(),
        HoldKind::Script if label.is_empty() => plural(n, "hold file"),
        HoldKind::Script if n > 1 => format!("{label} ({})", plural(n, "file")),
        HoldKind::Script => label.to_owned(),
        // `present` names Keep awake through `keep_awake_group`, with its time left.
        HoldKind::KeepAwake => "Keep awake".to_owned(),
    }
}

/// The Keep awake group; `Minutes(n)` is the minutes left.
pub fn keep_awake_group(left: KeepAwake) -> String {
    match left {
        KeepAwake::Minutes(minutes) => format!("Keep awake ({} left)", span(u64::from(minutes))),
        KeepAwake::UntilOff => "Keep awake (until turned off)".to_owned(),
        // `present` never asks: a Keep awake that is off holds nothing.
        KeepAwake::Off => "Keep awake".to_owned(),
    }
}

/// The groups cut from a full tooltip, as one more group.
pub fn more_groups(n: u32) -> String {
    format!("and {n} more")
}

/// The menu's status line: a menu text, so `&` is doubled and a reason shows as written, with no access key.
pub fn status_line(status: &Status, groups: &[String]) -> String {
    line(status, groups).replace('&', "&&")
}

/// The tooltip; `present` fits it to `TOOLTIP_MAX_UTF16` by dropping whole groups. Two lines: the app's name, since
/// the tray holds many icons, then the status line's words.
pub fn tooltip(status: &Status, groups: &[String]) -> String {
    format!("{NAME_LINE}{}", line(status, groups))
}

/// The tooltip of a dry-run tray: the mark goes on the name line, so a cut never takes it. `present` fitted the
/// tooltip without the mark, so the end may go here (dry run only, `p2-present` Found outside scope 4).
pub fn dry_run(tooltip: &str) -> String {
    let rest = tooltip.strip_prefix(NAME_LINE).unwrap_or(tooltip);
    fit(format!("Wakeshot (dry run)\n{rest}"), TOOLTIP_MAX_UTF16)
}

/// A notice's words and behavior. One `match` with no wildcard, so a new `Notice` does not compile until it has
/// words. Windows 11 shows a balloon as a banner headed with the app's name (research, "Balloon on Windows 11"), so
/// titles say what happened.
/// Title and body are cut to the balloon's limits as a last guard, for the values from outside (a version, an
/// agent's label, what held).
pub fn notice(notice: &Notice) -> NoticeText {
    let text = match notice {
        Notice::FirstRun => NoticeText {
            click: NoticeClick::OpenTaskbarSettings,
            ..info(
                "Wakeshot is running",
                "Its icon may hide under the ^ arrow on the taskbar. To keep it in view, click this notice, then \
                 turn on Wakeshot under \"Other system tray icons\".",
            )
        },
        Notice::UserChanged(key) => info(
            "Wakeshot keeps your change",
            format!(
                "You changed {}. Wakeshot leaves it as you set it until the current work ends.",
                setting(*key)
            ),
        ),
        Notice::RestoredByUser => info(
            "Your power settings are back",
            "Wakeshot put back your lid and sleep settings and leaves them alone until the current work ends. It \
             still keeps the PC awake for the work, but only while the lid is open.",
        ),
        // The battery floor chime plays with it, so the balloon is silent and two sounds never overlap.
        Notice::StepBack { floor_pct } => NoticeText {
            silent: true,
            ..warning(
                "Battery low: the PC may sleep now",
                format!(
                    "The battery is under {floor_pct}%, so Wakeshot stopped keeping the PC awake and put back your \
                     power settings. Plug in to keep it awake again."
                ),
            )
        },
        Notice::RequestsOnly => warning(
            "Wakeshot cannot change power settings",
            "Something, such as a company policy, blocked the change. Wakeshot still keeps the PC awake, but only \
             while the lid is open.",
        ),
        Notice::CannotHold => warning(
            "Wakeshot cannot keep the PC awake",
            "Windows did not accept Wakeshot's request, so the PC may sleep while your work runs.",
        ),
        // The log holds each write with its old value, the values to set by hand.
        Notice::RestoreFailed => NoticeText {
            click: NoticeClick::OpenLog,
            ..warning(
                "Could not put back your power settings",
                "Wakeshot keeps trying. Until it works, closing the lid may not sleep the PC, and on battery it may \
                 not sleep at all. Click to see your old values in the log.",
            )
        },
        Notice::HibernateFailed => warning(
            "Wakeshot could not hibernate the PC",
            "Wakeshot tried to hibernate the PC with the lid closed, but it did not work. Your power settings are \
             back, so Windows should sleep it at your usual sleep time.",
        ),
        // Raised at the first failure of a closed-lid lock, before its retry and the step back (architecture section
        // 8, "Lock fails"), and for a failed Keep screen on lock with the lid open, so it fits both.
        Notice::LockFailed => warning(
            "Wakeshot could not lock the PC",
            "Windows did not lock the PC. Lock it with Windows key + L. With the lid closed, Wakeshot tries once \
             more; if that fails, it stops keeping the PC awake and hibernates it after 5 minutes with no input.",
        ),
        Notice::ClosedLidReport { ran_ms, held_by } => {
            let lead = format!(
                "The PC kept running for {} with the lid closed.",
                span(ran_ms / MINUTE_MS)
            );
            let body = if held_by.is_empty() {
                lead
            } else {
                let room =
                    BODY_ENGLISH_UTF16.saturating_sub(units(&lead) + units(" What held it: ."));
                format!("{lead} What held it: {}.", names(held_by, room))
            };
            info("The PC ran with the lid closed", body)
        }
        Notice::UpdateWakeshot { agent } => warning(
            "Update Wakeshot",
            format!(
                "Wakeshot cannot fully read what {agent} is doing, most likely because {agent} was updated. Until \
                 you update Wakeshot, it may not keep the PC awake for {agent} as it should."
            ),
        ),
        // The log holds each value the reset found, and the journal saves before it the user's own.
        Notice::JournalLost { failed: true, .. } => NoticeText {
            click: NoticeClick::OpenLog,
            ..warning(
                "Check your lid and sleep settings",
                "Wakeshot could not read its record of your settings, and could not set them all back to Windows \
                 defaults. Set the lid and sleep settings by hand; click to see the old values in the log.",
            )
        },
        Notice::JournalLost { reset, .. } => {
            let lid = reset
                .iter()
                .any(|k| matches!(k, SettingKey::LidAc | SettingKey::LidDc));
            let sleep = reset.contains(&SettingKey::SleepDc);
            // Lid on AC and on battery are one setting to the user: "Closing the lid will make my PC".
            let (what, default, it) = match (lid, sleep) {
                (true, true) => (
                    "the lid and battery sleep settings",
                    "Windows defaults",
                    "them",
                ),
                (false, true) => ("the battery sleep setting", "the Windows default", "it"),
                _ => ("the lid setting", "the Windows default", "it"),
            };
            NoticeText {
                click: NoticeClick::OpenLog,
                ..warning(
                    "Power settings set to Windows defaults",
                    format!(
                        "Wakeshot could not read its record of your settings, so it set {what} to {default}. If you \
                         had your own, set {it} again. Click to see the old values in the log."
                    ),
                )
            }
        }
        // Silent: it comes when nobody has used the PC for a while, and a sound for an absent user helps nobody
        // (Phase 2 track note 11).
        Notice::IdleLocked => NoticeText {
            silent: true,
            ..info(
                "Wakeshot locked the PC",
                "Keep screen on is on, and the PC had no input for your usual sleep time, so Wakeshot locked it. \
                 Your work keeps running.",
            )
        },
        Notice::Updated { version } => NoticeText {
            silent: true,
            ..info(
                format!("Wakeshot updated to {version}"),
                "The new version is running.",
            )
        },
    };
    NoticeText {
        title: fit(text.title, TITLE_MAX_UTF16),
        body: fit(text.body, BODY_MAX_UTF16),
        ..text
    }
}

fn info(title: impl Into<String>, body: impl Into<String>) -> NoticeText {
    NoticeText {
        title: title.into(),
        body: body.into(),
        warning: false,
        silent: false,
        click: NoticeClick::Nothing,
    }
}

fn warning(title: impl Into<String>, body: impl Into<String>) -> NoticeText {
    NoticeText {
        warning: true,
        ..info(title, body)
    }
}

/// A setting in the user's words, not Windows' labels, which differ between Settings and Control Panel.
fn setting(key: SettingKey) -> &'static str {
    match key {
        SettingKey::LidAc => "what closing the lid does when plugged in",
        SettingKey::LidDc => "what closing the lid does on battery",
        SettingKey::SleepDc => "when the PC sleeps on battery",
    }
}

/// The status line's words. The head before the groups stays at most 28 units, so one 60 character reason and
/// "and N more" still fit the tooltip under every status (`tooltip_fits_for_any_holds`). That is why lid open only
/// (decision 29: requests only) replaces screen on rather than joining it: it changes what the lid does, and a
/// screen that stays on shows itself.
fn line(status: &Status, groups: &[String]) -> String {
    let holds = !groups.is_empty();
    let head = if status.stepped_back {
        "Battery low, not holding"
    } else {
        match (status.icon, holds) {
            (IconState::Holding, _) if status.requests_only => "Holding awake, lid open only",
            (IconState::Holding, _) if status.screen_on => "Holding awake, screen on",
            (IconState::Holding, _) => "Holding awake",
            // Holds count but no request is on: it failed (`CannotHold`).
            (IconState::Idle, true) => "Cannot hold awake",
            (IconState::Idle, false) => "Not holding, the PC sleeps as usual",
            // `present` shows Off only while nothing holds.
            (IconState::Off, true) => "Not holding",
            (IconState::Off, false) => "Automatic is off, the PC sleeps as usual",
        }
    };
    if holds {
        format!("{head}: {}", groups.join(", "))
    } else {
        head.to_owned()
    }
}

/// A Keep awake length in the menu's full words: "30 minutes", "1 hour", "1 hour 30 minutes".
fn length(minutes: u32) -> String {
    let (hours, minutes) = (u64::from(minutes / 60), u64::from(minutes % 60));
    match (hours, minutes) {
        (0, m) => plural(m, "minute"),
        (h, 0) => plural(h, "hour"),
        (h, m) => format!("{} {}", plural(h, "hour"), plural(m, "minute")),
    }
}

/// A length of time in the short words the tooltip has room for: "less than a minute", "45 min", "1 h",
/// "2 h 15 min", "3 days 4 h". Minutes are dropped beside days, where they say nothing.
fn span(minutes: u64) -> String {
    let (days, hours, minutes) = (minutes / (24 * 60), minutes / 60 % 24, minutes % 60);
    match (days, hours, minutes) {
        (0, 0, 0) => "less than a minute".to_owned(),
        (0, 0, m) => format!("{m} min"),
        (0, h, 0) => format!("{h} h"),
        (0, h, m) => format!("{h} h {m} min"),
        (d, 0, _) => plural(d, "day"),
        (d, h, _) => format!("{} {h} h", plural(d, "day")),
    }
}

/// What held, as a list within `room` UTF-16 units: "A", "A and B", "A, B and C", else as many whole names as fit
/// and "and N more", so a long reason never ends mid-word; "N holds" when not even one fits.
fn names(names: &[String], room: usize) -> String {
    (1..=names.len())
        .rev()
        .map(|keep| list(&names[..keep], names.len() - keep))
        .find(|text| units(text) <= room)
        .unwrap_or_else(|| plural(u64::try_from(names.len()).unwrap_or(u64::MAX), "hold"))
}

fn list(shown: &[String], more: usize) -> String {
    let more = (more > 0).then(|| format!("{more} more"));
    let parts: Vec<&str> = shown
        .iter()
        .map(String::as_str)
        .chain(more.as_deref())
        .collect();
    match parts.split_last() {
        Some((last, [])) => (*last).to_owned(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// "1 session", "2 sessions": every count word here takes a plain "s".
fn plural(n: u64, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

fn units(text: &str) -> usize {
    text.encode_utf16().count()
}

/// At most `max` UTF-16 units, cut on a character boundary.
fn fit(mut text: String, max: usize) -> String {
    let mut used = 0;
    let cut = text.char_indices().find(|&(_, c)| {
        used += c.len_utf16();
        used > max
    });
    if let Some((end, _)) = cut {
        text.truncate(end);
    }
    text
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use proptest::prelude::*;

    use super::*;
    use crate::present::KEEP_AWAKE_CHOICES;
    use crate::settings::SettingKey;

    const MINUTE: u64 = 60_000;
    /// `NOTIFYICONDATAW`: `szInfoTitle[64]` and `szInfo[256]`, each less the end mark.
    const TITLE_MAX: usize = 63;
    const BODY_MAX: usize = 255;
    /// The same page asks for at most 48 and 200 characters in English, to leave room for other languages.
    const TITLE_ENGLISH: usize = 48;
    const BODY_ENGLISH: usize = 200;

    fn fits(tooltip: &str) -> bool {
        units(tooltip) <= TOOLTIP_MAX_UTF16
    }

    fn status(icon: IconState) -> Status {
        Status {
            icon,
            stepped_back: false,
            requests_only: false,
            screen_on: false,
        }
    }

    /// Every combination, reachable from `present` or not, so no input gives broken words.
    fn every_status() -> Vec<Status> {
        let mut all = vec![];
        for icon in [IconState::Holding, IconState::Idle, IconState::Off] {
            for bits in 0..8u8 {
                all.push(Status {
                    icon,
                    stepped_back: bits & 1 != 0,
                    requests_only: bits & 2 != 0,
                    screen_on: bits & 4 != 0,
                });
            }
        }
        all
    }

    /// Every command the menu sends, both ways for the toggles.
    fn commands() -> Vec<MenuCommand> {
        let mut all = vec![MenuCommand::Automatic(true), MenuCommand::Automatic(false)];
        all.extend(KEEP_AWAKE_CHOICES.map(MenuCommand::KeepAwake));
        all.extend([
            MenuCommand::KeepScreenOn(true),
            MenuCommand::KeepScreenOn(false),
            MenuCommand::Restore,
            MenuCommand::OpenLog,
            MenuCommand::Quit,
        ]);
        all
    }

    /// The access keys of a menu text by the Win32 rule (`DrawText`): the character after a single `&`; `&&` shows
    /// one `&` and is no key.
    fn access_keys(text: &str) -> Vec<char> {
        let mut keys = vec![];
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '&' {
                match chars.next() {
                    Some('&') | None => {}
                    Some(key) => keys.extend(key.to_lowercase()),
                }
            }
        }
        keys
    }

    /// The menu's two levels as `present` builds them (its `the_menu_follows_the_sketch_with_each_command` test).
    fn menu_levels() -> [Vec<String>; 2] {
        let top = vec![
            action(MenuCommand::Automatic(false)),
            keep_awake_menu(),
            action(MenuCommand::Restore),
            action(MenuCommand::OpenLog),
            action(MenuCommand::Quit),
        ];
        let mut keep_awake: Vec<String> = KEEP_AWAKE_CHOICES
            .map(|choice| action(MenuCommand::KeepAwake(choice)))
            .to_vec();
        keep_awake.push(action(MenuCommand::KeepScreenOn(true)));
        [top, keep_awake]
    }

    /// One sample of every variant.
    fn samples() -> Vec<Notice> {
        vec![
            Notice::FirstRun,
            Notice::UserChanged(SettingKey::LidAc),
            Notice::RestoredByUser,
            Notice::StepBack { floor_pct: 20 },
            Notice::RequestsOnly,
            Notice::CannotHold,
            Notice::RestoreFailed,
            Notice::HibernateFailed,
            Notice::LockFailed,
            Notice::ClosedLidReport {
                ran_ms: 135 * MINUTE,
                held_by: vec!["Claude Code".to_owned(), "cargo".to_owned()],
            },
            Notice::UpdateWakeshot {
                agent: "Claude Code",
            },
            Notice::JournalLost {
                reset: KEYS.to_vec(),
                failed: false,
            },
            Notice::IdleLocked,
            Notice::Updated {
                version: "1.1.0".to_owned(),
            },
        ]
    }

    const KEYS: [SettingKey; 3] = [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc];

    /// No wildcard: a new variant fails to compile here until `samples` and the flags table below name it.
    fn variant(notice: &Notice) -> usize {
        match notice {
            Notice::FirstRun => 0,
            Notice::UserChanged(_) => 1,
            Notice::RestoredByUser => 2,
            Notice::StepBack { .. } => 3,
            Notice::RequestsOnly => 4,
            Notice::CannotHold => 5,
            Notice::RestoreFailed => 6,
            Notice::HibernateFailed => 7,
            Notice::LockFailed => 8,
            Notice::ClosedLidReport { .. } => 9,
            Notice::UpdateWakeshot { .. } => 10,
            Notice::JournalLost { .. } => 11,
            Notice::IdleLocked => 12,
            Notice::Updated { .. } => 13,
        }
    }

    const VARIANTS: usize = 14;

    /// Leaked once per call: `UpdateWakeshot` takes a `&'static str`, and tests are short.
    fn leak(text: String) -> &'static str {
        Box::leak(text.into_boxed_str())
    }

    #[test]
    fn every_notice_has_words_within_balloon_limits() {
        let samples = samples();
        let covered: BTreeSet<usize> = samples.iter().map(variant).collect();
        assert_eq!(covered, (0..VARIANTS).collect(), "one sample per variant");

        // (warning, silent, click) by variant: silent where a chime plays (the step back) or nobody is there to
        // hear (Keep screen on's idle lock) or the notice is a quiet one (`Updated`); the log for the two that leave
        // values to set by hand; the taskbar page for the first run.
        let flags = [
            (false, false, NoticeClick::OpenTaskbarSettings),
            (false, false, NoticeClick::Nothing),
            (false, false, NoticeClick::Nothing),
            (true, true, NoticeClick::Nothing),
            (true, false, NoticeClick::Nothing),
            (true, false, NoticeClick::Nothing),
            (true, false, NoticeClick::OpenLog),
            (true, false, NoticeClick::Nothing),
            (true, false, NoticeClick::Nothing),
            (false, false, NoticeClick::Nothing),
            (true, false, NoticeClick::Nothing),
            (true, false, NoticeClick::OpenLog),
            (false, true, NoticeClick::Nothing),
            (false, true, NoticeClick::Nothing),
        ];
        for sample in &samples {
            let text = notice(sample);
            // An empty `szInfo` removes a balloon instead of showing one (`NOTIFYICONDATAW`, `NIF_INFO`).
            assert!(
                !text.title.is_empty() && !text.body.is_empty(),
                "{sample:?}"
            );
            assert!(units(&text.title) <= TITLE_ENGLISH, "{:?}", text.title);
            assert!(units(&text.body) <= BODY_ENGLISH, "{:?}", text.body);
            assert_eq!(
                (text.warning, text.silent, text.click),
                flags[variant(sample)],
                "{sample:?}"
            );
        }

        // Each form of the lost-journal notice, not only the sample's, keeps to the English limits.
        let lost_forms = [
            (vec![SettingKey::LidAc], false),
            (vec![SettingKey::LidAc, SettingKey::LidDc], false),
            (vec![SettingKey::SleepDc], false),
            (vec![SettingKey::LidDc, SettingKey::SleepDc], false),
            (KEYS.to_vec(), true),
            (Vec::new(), true),
        ];
        for (reset, failed) in lost_forms {
            let text = notice(&Notice::JournalLost { reset, failed });
            assert!(units(&text.title) <= TITLE_ENGLISH, "{:?}", text.title);
            assert!(units(&text.body) <= BODY_ENGLISH, "{:?}", text.body);
            assert_eq!(text.click, NoticeClick::OpenLog);
        }

        // The words around user-given and system-given values stay within the hard limits, whatever the values.
        let wide = "\u{1F600}".repeat(60);
        let extremes = [
            Notice::StepBack { floor_pct: u8::MAX },
            Notice::ClosedLidReport {
                ran_ms: u64::MAX,
                held_by: vec![wide.clone(); 40],
            },
            Notice::ClosedLidReport {
                ran_ms: u64::MAX,
                held_by: (0..40).map(|i| format!("{i} {}", "x".repeat(57))).collect(),
            },
            Notice::UpdateWakeshot {
                agent: leak(wide.repeat(5)),
            },
            Notice::Updated {
                version: wide.clone(),
            },
            Notice::Updated {
                version: "1.10.0-beta.12+build.345".to_owned(),
            },
        ];
        for sample in &extremes {
            let text = notice(sample);
            assert!(units(&text.title) <= TITLE_MAX, "{:?}", text.title);
            assert!(units(&text.body) <= BODY_MAX, "{:?}", text.body);
            assert!(
                !text.title.is_empty() && !text.body.is_empty(),
                "{sample:?}"
            );
        }
    }

    #[test]
    fn menu_access_keys_are_unique_per_level() {
        for level in menu_levels() {
            let mut seen = BTreeSet::new();
            for text in &level {
                let keys = access_keys(text);
                assert_eq!(keys.len(), 1, "{text:?} needs exactly one access key");
                assert!(
                    seen.insert(keys[0]),
                    "{text:?} repeats the key {:?}",
                    keys[0]
                );
            }
        }
        // A toggle row carries the command it sends, the opposite of what it shows, so its words must not depend on
        // the bool (`p2-present` Found outside scope 3).
        assert_eq!(
            action(MenuCommand::Automatic(true)),
            action(MenuCommand::Automatic(false))
        );
        assert_eq!(
            action(MenuCommand::KeepScreenOn(true)),
            action(MenuCommand::KeepScreenOn(false))
        );
    }

    #[test]
    fn user_text_ampersand_is_doubled() {
        let holding = status(IconState::Holding);
        let group = hold_group(HoldKind::Script, "build & test", 1);
        // Plain where no `&` rule applies: the tooltip and `state.json` (`wakeshot status` prints it).
        assert_eq!(group, "build & test");
        assert_eq!(
            tooltip(&holding, std::slice::from_ref(&group)),
            "Wakeshot\nHolding awake: build & test"
        );
        // Doubled in the menu's status line, so it shows as written and gives no access key.
        let line = status_line(&holding, &[group]);
        assert_eq!(line, "Holding awake: build && test");
        assert!(access_keys(&line).is_empty());

        let doubled = hold_group(HoldKind::Script, "a && b &", 1);
        let line = status_line(&holding, &[doubled]);
        assert_eq!(line, "Holding awake: a &&&& b &&");
        assert!(access_keys(&line).is_empty());
    }

    proptest! {
        #[test]
        fn tooltip_fits_for_any_holds(
            status in any_status(),
            n in any::<u32>(),
            long in group(LONG_LABEL, 99, u32::MAX),
            few in 1..=99u32,
            short in prop::collection::vec(group(SHORT_LABEL, 999, 99 * 1440), 1..=2),
            more in 1..=999u32,
        ) {
            // `present` drops whole groups until the tooltip fits; with none left it shows only the count. That
            // always fits, so `present` never leaves the cut to the tray.
            prop_assert!(fits(&tooltip(&status, &[])));
            prop_assert!(fits(&tooltip(&status, &[more_groups(n)])));
            // Any one group the sources can give, a 60 character reason included, shows whole with "and N more".
            let one = [long, more_groups(few)];
            prop_assert!(fits(&tooltip(&status, &one)), "{:?}", tooltip(&status, &one));
            // Two everyday groups (an agent, a program, Keep awake) and "and N more" fit under any status.
            let mut two = short;
            two.push(more_groups(more));
            prop_assert!(fits(&tooltip(&status, &two)), "{:?}", tooltip(&status, &two));
            // A dry-run tray marks the fitted tooltip and still stays within the limit, mark first.
            let marked = dry_run(&tooltip(&status, &one));
            prop_assert!(fits(&marked) && marked.starts_with("Wakeshot (dry run)\n"), "{marked:?}");
        }
    }

    /// BMP text up to the sources' 60 character cut: Latin, Armenian and Cyrillic, `&` included.
    const LONG_LABEL: &str = "[a-zA-Z0-9 &._\u{0561}-\u{0586}\u{0430}-\u{044f}]{0,60}";
    /// An agent's label or a program name.
    const SHORT_LABEL: &str = "[a-zA-Z0-9 &._\u{0561}-\u{0586}\u{0430}-\u{044f}]{0,20}";

    fn any_status() -> impl Strategy<Value = Status> {
        (
            prop_oneof![
                Just(IconState::Holding),
                Just(IconState::Idle),
                Just(IconState::Off)
            ],
            any::<(bool, bool, bool)>(),
        )
            .prop_map(|(icon, (stepped_back, requests_only, screen_on))| Status {
                icon,
                stepped_back,
                requests_only,
                screen_on,
            })
    }

    /// A group as `present` makes one: a counted hold of any kind, or Keep awake with up to `max_minutes` left.
    fn group(
        label: &'static str,
        max_count: u32,
        max_minutes: u32,
    ) -> impl Strategy<Value = String> {
        let kind = prop_oneof![
            Just(HoldKind::Agent),
            Just(HoldKind::Run),
            Just(HoldKind::Script),
            Just(HoldKind::KeepAwake),
        ];
        let left = prop_oneof![
            Just(KeepAwake::Off),
            Just(KeepAwake::UntilOff),
            (0..=max_minutes).prop_map(KeepAwake::Minutes),
        ];
        prop_oneof![
            (
                kind,
                prop::string::string_regex(label).unwrap(),
                1..=max_count
            )
                .prop_map(|(kind, label, count)| hold_group(kind, &label, count)),
            left.prop_map(keep_awake_group),
        ]
    }

    #[test]
    fn durations_read_plainly() {
        let table = [
            (0, "less than a minute"),
            (1, "1 min"),
            (45, "45 min"),
            (60, "1 h"),
            (135, "2 h 15 min"),
            (24 * 60, "1 day"),
            (25 * 60 + 30, "1 day 1 h"),
            (2 * 24 * 60 + 3 * 60 + 59, "2 days 3 h"),
        ];
        for (minutes, words) in table {
            assert_eq!(span(minutes), words, "{minutes} min");
        }
        assert_eq!(
            keep_awake_group(KeepAwake::Minutes(25)),
            "Keep awake (25 min left)"
        );
        assert_eq!(
            keep_awake_group(KeepAwake::Minutes(90)),
            "Keep awake (1 h 30 min left)"
        );
        // The closed-lid report rounds down: 59.999 s did not make a minute.
        let report = notice(&Notice::ClosedLidReport {
            ran_ms: MINUTE - 1,
            held_by: vec![],
        });
        assert_eq!(
            report.body,
            "The PC kept running for less than a minute with the lid closed."
        );
    }

    #[test]
    fn counts_read_plainly() {
        let table = [
            (
                hold_group(HoldKind::Agent, "Claude Code", 1),
                "Claude Code (1 session)",
            ),
            (
                hold_group(HoldKind::Agent, "Claude Code", 2),
                "Claude Code (2 sessions)",
            ),
            (hold_group(HoldKind::Run, "cargo", 1), "cargo"),
            (hold_group(HoldKind::Run, "cargo", 2), "cargo (2 commands)"),
            (
                hold_group(HoldKind::Script, "nightly backup", 1),
                "nightly backup",
            ),
            (
                hold_group(HoldKind::Script, "nightly backup", 2),
                "nightly backup (2 files)",
            ),
            (more_groups(1), "and 1 more"),
            (more_groups(12), "and 12 more"),
        ];
        for (got, want) in table {
            assert_eq!(got, want);
        }
    }

    /// `present` passes an empty label for a hold file without a reason; each kind gets its own name.
    #[test]
    fn an_empty_label_gets_a_name_by_kind() {
        let table = [
            (hold_group(HoldKind::Agent, "", 1), "an agent (1 session)"),
            (hold_group(HoldKind::Run, "", 1), "wakeshot run"),
            (
                hold_group(HoldKind::Run, "", 3),
                "wakeshot run (3 commands)",
            ),
            (hold_group(HoldKind::Script, "", 1), "a hold file"),
            (hold_group(HoldKind::Script, "", 2), "2 hold files"),
            (hold_group(HoldKind::KeepAwake, "", 1), "Keep awake"),
        ];
        for (got, want) in table {
            assert_eq!(got, want);
        }
    }

    #[test]
    fn closed_lid_report_names_what_held_it() {
        let list = |n: usize| -> Vec<String> { (1..=n).map(|i| format!("hold {i}")).collect() };
        assert_eq!(names(&list(1), 200), "hold 1");
        assert_eq!(names(&list(2), 200), "hold 1 and hold 2");
        assert_eq!(names(&list(3), 200), "hold 1, hold 2 and hold 3");
        // Whole names only, then how many more.
        assert_eq!(names(&list(5), 25), "hold 1, hold 2 and 3 more");
        assert_eq!(names(&list(5), 24), "hold 1 and 4 more");
        assert_eq!(names(&list(5), 6), "5 holds");
        assert_eq!(names(&list(1), 3), "1 hold");
    }

    #[test]
    fn no_dashes_emojis_or_non_english_marks() {
        let mut all = vec![keep_awake_menu()];
        all.extend(commands().into_iter().map(action));
        let groups = [
            hold_group(HoldKind::Agent, "Claude Code", 2),
            hold_group(HoldKind::Run, "cargo", 2),
            hold_group(HoldKind::Script, "", 2),
            keep_awake_group(KeepAwake::Minutes(2 * 24 * 60 + 5)),
            keep_awake_group(KeepAwake::UntilOff),
            more_groups(3),
        ];
        all.extend(groups.iter().cloned());
        for status in every_status() {
            for shown in [&groups[..0], &groups[..]] {
                all.push(status_line(&status, shown));
                all.push(tooltip(&status, shown));
                all.push(dry_run(&tooltip(&status, shown)));
            }
        }
        let mut notices = samples();
        notices.extend([
            Notice::UserChanged(SettingKey::LidDc),
            Notice::UserChanged(SettingKey::SleepDc),
        ]);
        for sample in &notices {
            let text = notice(sample);
            all.extend([text.title, text.body]);
        }
        for text in &all {
            let bad = text.chars().find(|&c| {
                matches!(c, '\u{2013}' | '\u{2014}' | '\u{200D}' | '\u{FE0F}')
                    || ('\u{2600}'..='\u{27BF}').contains(&c)
                    || ('\u{1F000}'..='\u{1FAFF}').contains(&c)
                    // Plain ASCII only: no curly quotes, ellipsis or other marks a keyboard does not type.
                    || !(c == '\n' || c == ' ' || c.is_ascii_graphic())
            });
            assert_eq!(bad, None, "{text:?}");
        }
    }

    /// The exact words, so every wording change is a visible diff in review. Haik reads this list once
    /// (`p5-haik-check`).
    #[test]
    fn wording_snapshot() {
        let holding = status(IconState::Holding);
        let claude = hold_group(HoldKind::Agent, "Claude Code", 2);
        let cargo = hold_group(HoldKind::Run, "cargo", 1);
        let left = keep_awake_group(KeepAwake::Minutes(25));
        let two = [claude.clone(), cargo.clone()];
        let screen = Status {
            screen_on: true,
            ..holding
        };
        let lid_open = Status {
            requests_only: true,
            ..holding
        };
        let both = Status {
            screen_on: true,
            ..lid_open
        };
        let stepped_back = Status {
            stepped_back: true,
            ..status(IconState::Idle)
        };
        let idle = status(IconState::Idle);
        let off = status(IconState::Off);
        let text = |n: Notice| notice(&n);
        let first_run = text(Notice::FirstRun);
        let lid_ac = text(Notice::UserChanged(SettingKey::LidAc));
        let lid_dc = text(Notice::UserChanged(SettingKey::LidDc));
        let sleep_dc = text(Notice::UserChanged(SettingKey::SleepDc));
        let restored = text(Notice::RestoredByUser);
        let step_back = text(Notice::StepBack { floor_pct: 20 });
        let requests_only = text(Notice::RequestsOnly);
        let cannot_hold = text(Notice::CannotHold);
        let restore_failed = text(Notice::RestoreFailed);
        let hibernate_failed = text(Notice::HibernateFailed);
        let lock_failed = text(Notice::LockFailed);
        let report = text(Notice::ClosedLidReport {
            ran_ms: 135 * MINUTE,
            held_by: vec!["Claude Code".to_owned(), "cargo".to_owned()],
        });
        let report_many = text(Notice::ClosedLidReport {
            ran_ms: 3 * 24 * 60 * MINUTE + 5 * 60 * MINUTE,
            held_by: (1..=9).map(|i| format!("nightly backup {i}")).collect(),
        });
        let update = text(Notice::UpdateWakeshot {
            agent: "Claude Code",
        });
        let lost = |reset: &[SettingKey], failed| {
            text(Notice::JournalLost {
                reset: reset.to_vec(),
                failed,
            })
        };
        let lost_all = lost(&KEYS, false);
        let lost_lid = lost(&[SettingKey::LidDc], false);
        let lost_sleep = lost(&[SettingKey::SleepDc], false);
        let lost_failed = lost(&[SettingKey::LidAc], true);
        let idle_locked = text(Notice::IdleLocked);
        let updated = text(Notice::Updated {
            version: "1.1.0".to_owned(),
        });

        let table: Vec<(String, &str)> = vec![
            // Menu.
            (action(MenuCommand::Automatic(false)), "&Automatic"),
            (keep_awake_menu(), "&Keep awake"),
            (action(MenuCommand::KeepAwake(KeepAwake::Off)), "&Off"),
            (
                action(MenuCommand::KeepAwake(KeepAwake::Minutes(30))),
                "&30 minutes",
            ),
            (
                action(MenuCommand::KeepAwake(KeepAwake::Minutes(60))),
                "&1 hour",
            ),
            (
                action(MenuCommand::KeepAwake(KeepAwake::Minutes(120))),
                "&2 hours",
            ),
            (
                action(MenuCommand::KeepAwake(KeepAwake::Minutes(240))),
                "&4 hours",
            ),
            (
                action(MenuCommand::KeepAwake(KeepAwake::Minutes(1))),
                "&1 minute",
            ),
            (
                action(MenuCommand::KeepAwake(KeepAwake::Minutes(90))),
                "&1 hour 30 minutes",
            ),
            (
                action(MenuCommand::KeepAwake(KeepAwake::UntilOff)),
                "&Until I turn it off",
            ),
            (action(MenuCommand::KeepScreenOn(true)), "Keep &screen on"),
            (action(MenuCommand::Restore), "&Restore my power settings"),
            (action(MenuCommand::OpenLog), "&Open log"),
            (action(MenuCommand::Quit), "&Quit"),
            // Groups.
            (claude.clone(), "Claude Code (2 sessions)"),
            (cargo.clone(), "cargo"),
            (left.clone(), "Keep awake (25 min left)"),
            (
                keep_awake_group(KeepAwake::UntilOff),
                "Keep awake (until turned off)",
            ),
            (keep_awake_group(KeepAwake::Off), "Keep awake"),
            (more_groups(3), "and 3 more"),
            // Status line and tooltip.
            (
                status_line(&holding, &two),
                "Holding awake: Claude Code (2 sessions), cargo",
            ),
            (
                tooltip(&holding, &two),
                "Wakeshot\nHolding awake: Claude Code (2 sessions), cargo",
            ),
            (
                tooltip(&holding, &[claude.clone(), more_groups(2)]),
                "Wakeshot\nHolding awake: Claude Code (2 sessions), and 2 more",
            ),
            (
                tooltip(&screen, std::slice::from_ref(&left)),
                "Wakeshot\nHolding awake, screen on: Keep awake (25 min left)",
            ),
            (
                tooltip(&lid_open, std::slice::from_ref(&cargo)),
                "Wakeshot\nHolding awake, lid open only: cargo",
            ),
            (
                tooltip(&both, std::slice::from_ref(&left)),
                "Wakeshot\nHolding awake, lid open only: Keep awake (25 min left)",
            ),
            (tooltip(&holding, &[]), "Wakeshot\nHolding awake"),
            (
                tooltip(&idle, &[]),
                "Wakeshot\nNot holding, the PC sleeps as usual",
            ),
            (
                tooltip(&idle, std::slice::from_ref(&cargo)),
                "Wakeshot\nCannot hold awake: cargo",
            ),
            (
                tooltip(&off, &[]),
                "Wakeshot\nAutomatic is off, the PC sleeps as usual",
            ),
            (
                tooltip(&off, std::slice::from_ref(&cargo)),
                "Wakeshot\nNot holding: cargo",
            ),
            (
                tooltip(&stepped_back, std::slice::from_ref(&claude)),
                "Wakeshot\nBattery low, not holding: Claude Code (2 sessions)",
            ),
            (
                tooltip(&stepped_back, &[]),
                "Wakeshot\nBattery low, not holding",
            ),
            (
                dry_run(&tooltip(&holding, std::slice::from_ref(&cargo))),
                "Wakeshot (dry run)\nHolding awake: cargo",
            ),
            // Notices.
            (first_run.title, "Wakeshot is running"),
            (
                first_run.body,
                "Its icon may hide under the ^ arrow on the taskbar. To keep it in view, click this notice, then turn \
                 on Wakeshot under \"Other system tray icons\".",
            ),
            (lid_ac.title, "Wakeshot keeps your change"),
            (
                lid_ac.body,
                "You changed what closing the lid does when plugged in. Wakeshot leaves it as you set it until the \
                 current work ends.",
            ),
            (
                lid_dc.body,
                "You changed what closing the lid does on battery. Wakeshot leaves it as you set it until the current \
                 work ends.",
            ),
            (
                sleep_dc.body,
                "You changed when the PC sleeps on battery. Wakeshot leaves it as you set it until the current work \
                 ends.",
            ),
            (restored.title, "Your power settings are back"),
            (
                restored.body,
                "Wakeshot put back your lid and sleep settings and leaves them alone until the current work ends. It \
                 still keeps the PC awake for the work, but only while the lid is open.",
            ),
            (step_back.title, "Battery low: the PC may sleep now"),
            (
                step_back.body,
                "The battery is under 20%, so Wakeshot stopped keeping the PC awake and put back your power settings. \
                 Plug in to keep it awake again.",
            ),
            (requests_only.title, "Wakeshot cannot change power settings"),
            (
                requests_only.body,
                "Something, such as a company policy, blocked the change. Wakeshot still keeps the PC awake, but only \
                 while the lid is open.",
            ),
            (cannot_hold.title, "Wakeshot cannot keep the PC awake"),
            (
                cannot_hold.body,
                "Windows did not accept Wakeshot's request, so the PC may sleep while your work runs.",
            ),
            (
                restore_failed.title,
                "Could not put back your power settings",
            ),
            (
                restore_failed.body,
                "Wakeshot keeps trying. Until it works, closing the lid may not sleep the PC, and on battery it may \
                 not sleep at all. Click to see your old values in the log.",
            ),
            (
                hibernate_failed.title,
                "Wakeshot could not hibernate the PC",
            ),
            (
                hibernate_failed.body,
                "Wakeshot tried to hibernate the PC with the lid closed, but it did not work. Your power settings are \
                 back, so Windows should sleep it at your usual sleep time.",
            ),
            (lock_failed.title, "Wakeshot could not lock the PC"),
            (
                lock_failed.body,
                "Windows did not lock the PC. Lock it with Windows key + L. With the lid closed, Wakeshot tries \
                 once more; if that fails, it stops keeping the PC awake and hibernates it after 5 minutes with no \
                 input.",
            ),
            (report.title, "The PC ran with the lid closed"),
            (
                report.body,
                "The PC kept running for 2 h 15 min with the lid closed. What held it: Claude Code and cargo.",
            ),
            (
                report_many.body,
                "The PC kept running for 3 days 5 h with the lid closed. What held it: nightly backup 1, nightly \
                 backup 2, nightly backup 3, nightly backup 4, nightly backup 5, nightly backup 6 and 3 more.",
            ),
            (update.title, "Update Wakeshot"),
            (
                update.body,
                "Wakeshot cannot fully read what Claude Code is doing, most likely because Claude Code was updated. \
                 Until you update Wakeshot, it may not keep the PC awake for Claude Code as it should.",
            ),
            (lost_all.title, "Power settings set to Windows defaults"),
            (
                lost_all.body,
                "Wakeshot could not read its record of your settings, so it set the lid and battery sleep settings to \
                 Windows defaults. If you had your own, set them again. Click to see the old values in the log.",
            ),
            (
                lost_lid.body,
                "Wakeshot could not read its record of your settings, so it set the lid setting to the Windows \
                 default. If you had your own, set it again. Click to see the old values in the log.",
            ),
            (
                lost_sleep.body,
                "Wakeshot could not read its record of your settings, so it set the battery sleep setting to the \
                 Windows default. If you had your own, set it again. Click to see the old values in the log.",
            ),
            (lost_failed.title, "Check your lid and sleep settings"),
            (
                lost_failed.body,
                "Wakeshot could not read its record of your settings, and could not set them all back to Windows \
                 defaults. Set the lid and sleep settings by hand; click to see the old values in the log.",
            ),
            (idle_locked.title, "Wakeshot locked the PC"),
            (
                idle_locked.body,
                "Keep screen on is on, and the PC had no input for your usual sleep time, so Wakeshot locked it. Your \
                 work keeps running.",
            ),
            (updated.title, "Wakeshot updated to 1.1.0"),
            (updated.body, "The new version is running."),
        ];
        let wrong: Vec<&(String, &str)> = table.iter().filter(|(got, want)| got != want).collect();
        assert!(wrong.is_empty(), "(got, want): {wrong:#?}");
    }
}
