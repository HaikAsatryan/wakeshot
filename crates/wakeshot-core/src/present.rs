//! `TrayView`, `MenuModel` and `StateFile` from the core's state; every word comes from `text`.
//!
//! Owner: `p2-present`.
//!
//! The core fills a [`Snapshot`] at each change the user can see and calls [`build`]; when it calls it, and the
//! `state.json` debounce, are the core's (`p2-core-holds`). This module decides only structure: the hold groups and
//! their order, the menu rows with their checks and enabled states, and the icon state (track notes 13 and 16).
//!
//! One list of groups serves three places, so `text` words a group once for all of them: the tooltip shows as many
//! whole groups as fit in [`TOOLTIP_MAX_UTF16`] plus `text::more_groups` for the rest; the menu's status line shows
//! the same ones, so many long reasons never make the menu wide; `state.json` gets the whole list. A toggle row
//! carries the command it sends, the opposite of what it shows, so `text::action` gets `Automatic(false)` for a
//! checked Automatic.

use std::collections::BTreeMap;

use crate::files::StateFile;
use crate::holds::HoldKind;
use crate::process::ProcessRef;
use crate::settings::OwnedKey;
use crate::text::{self, Status, TOOLTIP_MAX_UTF16};
use crate::time::Now;
use crate::view::{IconState, KeepAwake, MenuCommand, MenuItem, MenuModel, TrayView};

/// The Keep awake submenu's choices in menu order (product UX). The CLI's `30m|1h|2h|4h|on|off` are the same values.
pub const KEEP_AWAKE_CHOICES: [KeepAwake; 6] = [
    KeepAwake::Off,
    KeepAwake::Minutes(30),
    KeepAwake::Minutes(60),
    KeepAwake::Minutes(120),
    KeepAwake::Minutes(240),
    KeepAwake::UntilOff,
];

const MINUTE_MS: u64 = 60_000;

/// One hold that counts now, as the tray names it.
///
/// Owner: `p2-present`. The core fills one per counted agent session and hold file, from the policy's answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holder {
    pub kind: HoldKind,
    /// The agent's label (`SessionFacts::label`) or the hold file's reason, shown as given (the sources cut a reason
    /// to 60 characters). `None`: a hold file with no reason; `text::hold_group` then gets an empty label and words a
    /// fallback. Not read for `KeepAwake`, which `text::keep_awake_group` names.
    pub label: Option<String>,
    /// The hold file's `expires`. Read only for `KeepAwake`, for its time left.
    pub expires_wall_ms: Option<i64>,
}

/// The core's state as the tray and `state.json` show it, rebuilt whole at each change, like `TrayView`.
///
/// Owner: `p2-present`; the core cards fill it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub automatic: bool,
    /// Every hold that counts now, of any kind, also while stepped back. In any order: `build` sorts.
    pub holds: Vec<Holder>,
    /// The Keep awake row the menu checks: the length picked, one of [`KEEP_AWAKE_CHOICES`], not the time left (that
    /// comes from the hold's `expires`). For a hold file the tray did not write, see [`keep_awake_choice`].
    pub keep_awake: KeepAwake,
    /// Keep screen on. Shown only while `keep_awake` is not `Off` (track note 16).
    pub screen_on: bool,
    /// The system request is on now. False while stepped back, after a failed request and while nothing holds.
    pub requests_on: bool,
    /// This episode holds with power requests only (decision 29); for `text::Status`.
    pub requests_only: bool,
    /// Stepped back at the battery floor.
    pub stepped_back: bool,
    /// The keys of the core's journal mirror (known keys only). In any order: `build` sorts and drops repeats.
    pub overridden: Vec<OwnedKey>,
    /// This tray (`Started.tray`), so `wakeshot status` can tell a live tray from a stale file.
    pub tray: ProcessRef,
}

/// The tray's view and `state.json` for one snapshot. Pure, so the core can compare with the last answer and send
/// only a change. `now` gives Keep awake's minutes left, rounded up; they change once a minute, so a core that
/// rebuilds on each safety tick shows them at most one tick late.
pub fn build(snapshot: &Snapshot, now: Now) -> (TrayView, StateFile) {
    let icon = icon(snapshot);
    let screen_on = snapshot.screen_on && snapshot.keep_awake != KeepAwake::Off;
    let status = Status {
        icon,
        stepped_back: snapshot.stepped_back,
        requests_only: snapshot.requests_only,
        screen_on,
    };
    let groups = groups(&snapshot.holds, now);
    let (tooltip, shown) = fit_tooltip(&status, &groups);
    let status_line = MenuItem::Label {
        text: text::status_line(&status, &shown),
        icon: Some(icon),
    };
    let menu = menu(snapshot, screen_on, status_line);
    let view = TrayView {
        icon,
        tooltip,
        menu,
    };
    (view, state_file(snapshot, groups))
}

/// The Keep awake choice a hold file stands for when the tray did not write it (`wakeshot keep-awake`, a script):
/// `UntilOff` with no `expires`, else the shortest menu length that covers the time left, else the minutes left
/// (then the menu checks no length). Exact only for a new file, since the CLI writes now plus the length: call it
/// when the file appears or its `expires` changes, and keep the answer.
pub fn keep_awake_choice(expires_wall_ms: Option<i64>, now: Now) -> KeepAwake {
    let Some(expires) = expires_wall_ms else {
        return KeepAwake::UntilOff;
    };
    let left = minutes_left(expires, now);
    KEEP_AWAKE_CHOICES
        .into_iter()
        .find(|choice| matches!(choice, KeepAwake::Minutes(n) if *n >= left))
        .unwrap_or(KeepAwake::Minutes(left))
}

/// Rounded up and at least 1: a hold that still counts has time left, and 0 would read as already over.
fn minutes_left(expires_wall_ms: i64, now: Now) -> u32 {
    let ms = u64::try_from(expires_wall_ms.saturating_sub(now.wall_ms)).unwrap_or(0);
    u32::try_from(ms.div_ceil(MINUTE_MS))
        .unwrap_or(u32::MAX)
        .max(1)
}

/// Track note 16: stepped back shows Idle, with its reason in the tooltip; so does a hold whose request failed, since
/// nothing holds the PC then (decision 0).
fn icon(snapshot: &Snapshot) -> IconState {
    let holds = !snapshot.holds.is_empty();
    if holds && snapshot.requests_on {
        IconState::Holding
    } else if !holds && !snapshot.automatic {
        IconState::Off
    } else {
        IconState::Idle
    }
}

/// In `HoldKind` order (agents first, the product's main job; Keep awake last), then by label, so the words stay put
/// while holds come and go in any order.
fn groups(holds: &[Holder], now: Now) -> Vec<String> {
    let mut counts: BTreeMap<(HoldKind, Option<&str>), u32> = BTreeMap::new();
    let mut keep_awake = Vec::new();
    for hold in holds {
        if hold.kind == HoldKind::KeepAwake {
            keep_awake.push(hold.expires_wall_ms);
        } else {
            let count = counts
                .entry((hold.kind, hold.label.as_deref()))
                .or_default();
            *count = count.saturating_add(1);
        }
    }
    let mut groups: Vec<String> = counts
        .into_iter()
        .map(|((kind, label), count)| text::hold_group(kind, label.unwrap_or_default(), count))
        .collect();
    if !keep_awake.is_empty() {
        // Only `keep-awake.json` gives this kind, so there is one; should a caller pass more, the longest wins, so
        // the time left never reads short.
        let longest = keep_awake
            .into_iter()
            .try_fold(i64::MIN, |longest, expires| Some(longest.max(expires?)));
        let left = longest.map_or(KeepAwake::UntilOff, |expires| {
            KeepAwake::Minutes(minutes_left(expires, now))
        });
        groups.push(text::keep_awake_group(left));
    }
    groups
}

/// Drops whole groups from the end until the tooltip fits, and names how many it dropped, so no group is cut in the
/// middle. If even that does not fit, the tray's own cut is the last guard. Gives the tooltip and the groups it
/// shows.
fn fit_tooltip(status: &Status, groups: &[String]) -> (String, Vec<String>) {
    let mut shown = groups.to_vec();
    let mut tooltip = text::tooltip(status, &shown);
    let mut keep = groups.len();
    while keep > 0 && tooltip.encode_utf16().count() > TOOLTIP_MAX_UTF16 {
        keep -= 1;
        shown.truncate(keep);
        let dropped = u32::try_from(groups.len() - keep).unwrap_or(u32::MAX);
        shown.push(text::more_groups(dropped));
        tooltip = text::tooltip(status, &shown);
    }
    (tooltip, shown)
}

/// The product's menu sketch (decision 33), and nothing it rules out (decision 39).
fn menu(snapshot: &Snapshot, screen_on: bool, status_line: MenuItem) -> MenuModel {
    let keep_awake_on = snapshot.keep_awake != KeepAwake::Off;
    let mut keep_awake: Vec<MenuItem> = KEEP_AWAKE_CHOICES
        .into_iter()
        .map(|choice| {
            action(
                MenuCommand::KeepAwake(choice),
                choice == snapshot.keep_awake,
                true,
            )
        })
        .collect();
    // Keep screen on is an on and off option, not a seventh length; a line keeps it from reading as one.
    keep_awake.push(MenuItem::Separator);
    keep_awake.push(action(
        MenuCommand::KeepScreenOn(!screen_on),
        screen_on,
        keep_awake_on,
    ));
    let automatic = snapshot.automatic;
    MenuModel {
        items: vec![
            status_line,
            MenuItem::Separator,
            action(MenuCommand::Automatic(!automatic), automatic, true),
            MenuItem::Submenu {
                text: text::keep_awake_menu(),
                items: keep_awake,
            },
            MenuItem::Separator,
            // Always enabled: it is the user's way out, and it also stops overriding until the work ends (decision
            // 26), which counts even before the first write.
            action(MenuCommand::Restore, false, true),
            action(MenuCommand::OpenLog, false, true),
            action(MenuCommand::Quit, false, true),
        ],
    }
}

fn action(command: MenuCommand, checked: bool, enabled: bool) -> MenuItem {
    MenuItem::Action {
        text: text::action(command),
        command,
        checked,
        enabled,
    }
}

fn state_file(snapshot: &Snapshot, holding: Vec<String>) -> StateFile {
    let mut overridden = snapshot.overridden.clone();
    // One order, so a mirror that lists the same keys in another order gives the same file.
    overridden.sort_unstable();
    overridden.dedup();
    StateFile {
        version: StateFile::VERSION,
        pid: snapshot.tray.pid,
        proc_start: snapshot.tray.start,
        automatic: snapshot.automatic,
        holding,
        overridden,
        stepped_back: snapshot.stepped_back,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::{Read, read};
    use crate::process::FileTime;
    use crate::settings::{Guid, SettingKey};

    const NOW: Now = Now {
        wall_ms: 1_767_225_600_000,
        mono_ms: 42,
    };
    const MINUTE: i64 = 60_000;
    const TRAY: ProcessRef = ProcessRef {
        pid: 9120,
        start: FileTime(134_381_211_391_959_336),
    };
    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const POWER_SAVER: Guid = Guid(0xa1841308_3541_4fab_bc81_f71556f20b4a);

    /// Automatic on, nothing holds: the state right after a normal start.
    fn idle() -> Snapshot {
        Snapshot {
            automatic: true,
            holds: vec![],
            keep_awake: KeepAwake::Off,
            screen_on: false,
            requests_on: false,
            requests_only: false,
            stepped_back: false,
            overridden: vec![],
            tray: TRAY,
        }
    }

    /// `holds` count and the request is on, as the core gives it while holding.
    fn holding(holds: Vec<Holder>) -> Snapshot {
        Snapshot {
            holds,
            requests_on: true,
            ..idle()
        }
    }

    fn agent(label: &str) -> Holder {
        Holder {
            kind: HoldKind::Agent,
            label: Some(label.to_owned()),
            expires_wall_ms: None,
        }
    }

    fn file(kind: HoldKind, reason: Option<&str>) -> Holder {
        Holder {
            kind,
            label: reason.map(str::to_owned),
            expires_wall_ms: None,
        }
    }

    fn keep_awake(expires_wall_ms: Option<i64>) -> Holder {
        Holder {
            kind: HoldKind::KeepAwake,
            label: None,
            expires_wall_ms,
        }
    }

    fn status(icon: IconState) -> Status {
        Status {
            icon,
            stepped_back: false,
            requests_only: false,
            screen_on: false,
        }
    }

    fn fits(tooltip: &str) -> bool {
        tooltip.encode_utf16().count() <= TOOLTIP_MAX_UTF16
    }

    /// The groups a tooltip shows when it keeps the first `keep` whole: those, then one `more_groups` for the rest.
    fn cut(groups: &[String], keep: usize) -> Vec<String> {
        let mut shown = groups[..keep].to_vec();
        if keep < groups.len() {
            let more = u32::try_from(groups.len() - keep).unwrap();
            shown.push(text::more_groups(more));
        }
        shown
    }

    /// Checks that the view names `groups` under `status`: the tooltip fits and shows the longest run of whole groups
    /// that fits plus how many more, and the status line names the same and carries the tray icon's state. Gives how
    /// many groups it shows whole. Built only from `text`, so a wording change never breaks it.
    fn assert_names(view: &TrayView, status: &Status, groups: &[String]) -> usize {
        let keep = (0..=groups.len())
            .rev()
            .find(|&keep| text::tooltip(status, &cut(groups, keep)) == view.tooltip)
            .unwrap_or_else(|| panic!("{:?} is no cut of {groups:?}", view.tooltip));
        assert!(fits(&view.tooltip), "{:?} is over the limit", view.tooltip);
        if keep < groups.len() {
            let longer = text::tooltip(status, &cut(groups, keep + 1));
            assert!(
                !fits(&longer),
                "{longer:?} fits, so the cut dropped too much"
            );
        }
        let line = text::status_line(status, &cut(groups, keep));
        assert_eq!(view.icon, status.icon);
        assert_eq!(
            view.menu.items[0],
            MenuItem::Label {
                text: line,
                icon: Some(status.icon),
            }
        );
        keep
    }

    fn state_of(snapshot: &Snapshot, holding: Vec<String>) -> StateFile {
        StateFile {
            version: StateFile::VERSION,
            pid: TRAY.pid,
            proc_start: TRAY.start,
            automatic: snapshot.automatic,
            holding,
            overridden: vec![],
            stepped_back: snapshot.stepped_back,
        }
    }

    fn submenu(view: &TrayView) -> &[MenuItem] {
        match &view.menu.items[3] {
            MenuItem::Submenu { items, .. } => items,
            other => panic!("item 3 is {other:?}, not the Keep awake submenu"),
        }
    }

    fn checked(items: &[MenuItem]) -> Vec<MenuCommand> {
        items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action {
                    command,
                    checked: true,
                    ..
                } => Some(*command),
                _ => None,
            })
            .collect()
    }

    fn item(command: MenuCommand, checked: bool, enabled: bool) -> MenuItem {
        MenuItem::Action {
            text: text::action(command),
            command,
            checked,
            enabled,
        }
    }

    /// The Keep screen on row, the submenu's last.
    fn screen_row(view: &TrayView) -> &MenuItem {
        submenu(view).last().unwrap()
    }

    #[test]
    fn nothing_holds_with_automatic_on_gives_idle_and_no_groups() {
        let snapshot = idle();
        let (view, state) = build(&snapshot, NOW);
        assert_eq!(view.icon, IconState::Idle);
        assert_eq!(assert_names(&view, &status(IconState::Idle), &[]), 0);
        assert_eq!(state, state_of(&snapshot, vec![]));
    }

    #[test]
    fn nothing_holds_with_automatic_off_gives_off() {
        let snapshot = Snapshot {
            automatic: false,
            ..idle()
        };
        let (view, state) = build(&snapshot, NOW);
        assert_eq!(view.icon, IconState::Off);
        assert_eq!(assert_names(&view, &status(IconState::Off), &[]), 0);
        assert_eq!(state, state_of(&snapshot, vec![]));
        assert!(!state.automatic);
    }

    #[test]
    fn one_hold_with_the_request_on_gives_holding_and_one_group() {
        let snapshot = holding(vec![agent("Claude Code")]);
        let (view, state) = build(&snapshot, NOW);
        assert_eq!(view.icon, IconState::Holding);
        let groups = vec![text::hold_group(HoldKind::Agent, "Claude Code", 1)];
        assert_eq!(assert_names(&view, &status(IconState::Holding), &groups), 1);
        assert_eq!(state, state_of(&snapshot, groups));
    }

    #[test]
    fn keep_awake_with_automatic_off_gives_holding() {
        let snapshot = Snapshot {
            automatic: false,
            keep_awake: KeepAwake::UntilOff,
            ..holding(vec![keep_awake(None)])
        };
        let (view, state) = build(&snapshot, NOW);
        assert_eq!(view.icon, IconState::Holding);
        let groups = vec![text::keep_awake_group(KeepAwake::UntilOff)];
        assert_eq!(assert_names(&view, &status(IconState::Holding), &groups), 1);
        assert_eq!(state, state_of(&snapshot, groups));
    }

    /// Track note 16: stepped back shows Idle, and the tooltip gets the reason through `Status`.
    #[test]
    fn stepped_back_gives_idle_and_still_names_the_holds() {
        let snapshot = Snapshot {
            stepped_back: true,
            requests_on: false,
            ..holding(vec![agent("Claude Code")])
        };
        let (view, state) = build(&snapshot, NOW);
        assert_eq!(view.icon, IconState::Idle);
        let groups = vec![text::hold_group(HoldKind::Agent, "Claude Code", 1)];
        let stepped_back = Status {
            stepped_back: true,
            ..status(IconState::Idle)
        };
        assert_eq!(assert_names(&view, &stepped_back, &groups), 1);
        assert_eq!(state, state_of(&snapshot, groups));
        assert!(state.stepped_back);
    }

    /// A request that failed (`CannotHold`) holds nothing, so the icon must not say it does (decision 0).
    #[test]
    fn a_hold_without_the_request_gives_idle() {
        let snapshot = Snapshot {
            requests_on: false,
            ..holding(vec![file(HoldKind::Run, Some("cargo"))])
        };
        let (view, state) = build(&snapshot, NOW);
        assert_eq!(view.icon, IconState::Idle);
        let groups = vec![text::hold_group(HoldKind::Run, "cargo", 1)];
        assert_eq!(assert_names(&view, &status(IconState::Idle), &groups), 1);
        assert_eq!(state, state_of(&snapshot, groups));
    }

    #[test]
    fn requests_only_goes_to_the_status() {
        let snapshot = Snapshot {
            requests_only: true,
            ..holding(vec![agent("Codex")])
        };
        let (view, _) = build(&snapshot, NOW);
        let groups = vec![text::hold_group(HoldKind::Agent, "Codex", 1)];
        let requests_only = Status {
            requests_only: true,
            ..status(IconState::Holding)
        };
        assert_eq!(assert_names(&view, &requests_only, &groups), 1);
    }

    #[test]
    fn two_claude_sessions_make_one_group_of_two() {
        let snapshot = holding(vec![agent("Claude Code"), agent("Claude Code")]);
        let (view, state) = build(&snapshot, NOW);
        let groups = vec![text::hold_group(HoldKind::Agent, "Claude Code", 2)];
        assert_eq!(assert_names(&view, &status(IconState::Holding), &groups), 1);
        assert_eq!(state.holding, groups);
    }

    #[test]
    fn many_holds_group_by_kind_then_label_whatever_the_input_order() {
        let holds = vec![
            file(HoldKind::Script, Some("nightly backup")),
            agent("Codex"),
            keep_awake(Some(NOW.wall_ms + 90 * MINUTE)),
            agent("Claude Code"),
            file(HoldKind::Run, Some("cargo")),
            agent("Claude Code"),
            file(HoldKind::Run, Some("cargo")),
            file(HoldKind::Script, None),
        ];
        let groups = vec![
            text::hold_group(HoldKind::Agent, "Claude Code", 2),
            text::hold_group(HoldKind::Agent, "Codex", 1),
            text::hold_group(HoldKind::Run, "cargo", 2),
            text::hold_group(HoldKind::Script, "", 1),
            text::hold_group(HoldKind::Script, "nightly backup", 1),
            text::keep_awake_group(KeepAwake::Minutes(90)),
        ];
        let snapshot = Snapshot {
            keep_awake: KeepAwake::Minutes(120),
            ..holding(holds.clone())
        };
        let (view, state) = build(&snapshot, NOW);
        assert_names(&view, &status(IconState::Holding), &groups);
        assert_eq!(state, state_of(&snapshot, groups));

        let reversed = Snapshot {
            holds: holds.into_iter().rev().collect(),
            ..snapshot
        };
        assert_eq!(build(&reversed, NOW), (view, state));
    }

    /// `text::hold_group` words the fallback for a hold file that gave no reason; it gets an empty label.
    #[test]
    fn a_hold_file_without_a_reason_gets_an_empty_label() {
        let snapshot = holding(vec![file(HoldKind::Script, None)]);
        let (view, state) = build(&snapshot, NOW);
        let groups = vec![text::hold_group(HoldKind::Script, "", 1)];
        assert_eq!(assert_names(&view, &status(IconState::Holding), &groups), 1);
        assert_eq!(state.holding, groups);
    }

    /// Sources cut a reason to 60 characters; that long a reason still shows whole.
    #[test]
    fn a_long_reason_shows_whole() {
        let reason: String = "nightly backup ".repeat(4).chars().take(60).collect();
        assert_eq!(reason.chars().count(), 60);
        let snapshot = holding(vec![file(HoldKind::Script, Some(&reason))]);
        let (view, state) = build(&snapshot, NOW);
        let groups = vec![text::hold_group(HoldKind::Script, &reason, 1)];
        assert_eq!(assert_names(&view, &status(IconState::Holding), &groups), 1);
        assert_eq!(state.holding, groups);
    }

    #[test]
    fn a_tooltip_over_the_limit_drops_whole_groups_and_counts_them() {
        let reasons: Vec<String> = (0..6).map(|i| format!("{i} {}", "x".repeat(58))).collect();
        let holds = reasons
            .iter()
            .map(|r| file(HoldKind::Script, Some(r)))
            .collect();
        let snapshot = holding(holds);
        let (view, state) = build(&snapshot, NOW);
        let groups: Vec<String> = reasons
            .iter()
            .map(|r| text::hold_group(HoldKind::Script, r, 1))
            .collect();
        let shown = assert_names(&view, &status(IconState::Holding), &groups);
        assert!(shown < groups.len(), "nothing was cut");
        assert_eq!(state.holding, groups, "state.json keeps every group");
    }

    #[test]
    fn keep_awake_shows_the_minutes_left_rounded_up() {
        for (left_ms, minutes) in [
            (24 * MINUTE + 30_000, 25),
            (30 * MINUTE, 30),
            (MINUTE + 1, 2),
        ] {
            let snapshot = Snapshot {
                keep_awake: KeepAwake::Minutes(30),
                ..holding(vec![keep_awake(Some(NOW.wall_ms + left_ms))])
            };
            let (view, state) = build(&snapshot, NOW);
            let groups = vec![text::keep_awake_group(KeepAwake::Minutes(minutes))];
            assert_eq!(assert_names(&view, &status(IconState::Holding), &groups), 1);
            assert_eq!(state.holding, groups, "{left_ms} ms left");
        }
    }

    /// A hold that still counts has time left, so its last moments never read as 0 minutes.
    #[test]
    fn keep_awake_at_its_last_moment_shows_one_minute() {
        for left_ms in [1, 0, -MINUTE] {
            let snapshot = Snapshot {
                keep_awake: KeepAwake::Minutes(30),
                ..holding(vec![keep_awake(Some(NOW.wall_ms + left_ms))])
            };
            let (_, state) = build(&snapshot, NOW);
            let groups = vec![text::keep_awake_group(KeepAwake::Minutes(1))];
            assert_eq!(state.holding, groups, "{left_ms} ms left");
        }
    }

    #[test]
    fn keep_awake_without_expires_shows_until_off() {
        let snapshot = Snapshot {
            keep_awake: KeepAwake::UntilOff,
            ..holding(vec![keep_awake(None)])
        };
        let (_, state) = build(&snapshot, NOW);
        assert_eq!(state.holding, [text::keep_awake_group(KeepAwake::UntilOff)]);
    }

    #[test]
    fn each_keep_awake_choice_checks_only_itself() {
        for choice in KEEP_AWAKE_CHOICES {
            let snapshot = match choice {
                KeepAwake::Off => idle(),
                KeepAwake::UntilOff => holding(vec![keep_awake(None)]),
                KeepAwake::Minutes(n) => {
                    holding(vec![keep_awake(Some(NOW.wall_ms + i64::from(n) * MINUTE))])
                }
            };
            let snapshot = Snapshot {
                keep_awake: choice,
                ..snapshot
            };
            let (view, _) = build(&snapshot, NOW);
            assert_eq!(
                checked(submenu(&view)),
                [MenuCommand::KeepAwake(choice)],
                "{choice:?}"
            );
        }
    }

    /// A length only a script could set (no menu item has it) checks no length rather than a wrong one.
    #[test]
    fn a_length_the_menu_does_not_offer_checks_no_length() {
        let snapshot = Snapshot {
            keep_awake: KeepAwake::Minutes(45),
            ..holding(vec![keep_awake(Some(NOW.wall_ms + 45 * MINUTE))])
        };
        let (view, _) = build(&snapshot, NOW);
        assert_eq!(checked(submenu(&view)), []);
    }

    /// Track note 16: Keep screen on works only with Keep awake, so it is greyed out and shows off without it.
    #[test]
    fn keep_screen_on_is_disabled_while_keep_awake_is_off() {
        let snapshot = Snapshot {
            screen_on: true,
            ..idle()
        };
        let (view, _) = build(&snapshot, NOW);
        assert_eq!(
            screen_row(&view),
            &item(MenuCommand::KeepScreenOn(true), false, false)
        );
        assert_names(&view, &status(IconState::Idle), &[]);
    }

    #[test]
    fn keep_screen_on_is_enabled_and_toggles_while_keep_awake_is_on() {
        for screen_on in [false, true] {
            let snapshot = Snapshot {
                keep_awake: KeepAwake::UntilOff,
                screen_on,
                ..holding(vec![keep_awake(None)])
            };
            let (view, _) = build(&snapshot, NOW);
            assert_eq!(
                screen_row(&view),
                &item(MenuCommand::KeepScreenOn(!screen_on), screen_on, true)
            );
            let groups = vec![text::keep_awake_group(KeepAwake::UntilOff)];
            let shown = Status {
                screen_on,
                ..status(IconState::Holding)
            };
            assert_names(&view, &shown, &groups);
        }
    }

    /// The product's menu sketch (decision 33), each row with the command it sends, and nothing else (decision 39).
    #[test]
    fn the_menu_follows_the_sketch_with_each_command() {
        let (view, _) = build(&idle(), NOW);
        let keep_awake = |minutes| {
            item(
                MenuCommand::KeepAwake(KeepAwake::Minutes(minutes)),
                false,
                true,
            )
        };
        let expected = MenuModel {
            items: vec![
                MenuItem::Label {
                    text: text::status_line(&status(IconState::Idle), &[]),
                    icon: Some(IconState::Idle),
                },
                MenuItem::Separator,
                item(MenuCommand::Automatic(false), true, true),
                MenuItem::Submenu {
                    text: text::keep_awake_menu(),
                    items: vec![
                        item(MenuCommand::KeepAwake(KeepAwake::Off), true, true),
                        keep_awake(30),
                        keep_awake(60),
                        keep_awake(120),
                        keep_awake(240),
                        item(MenuCommand::KeepAwake(KeepAwake::UntilOff), false, true),
                        MenuItem::Separator,
                        item(MenuCommand::KeepScreenOn(true), false, false),
                    ],
                },
                MenuItem::Separator,
                item(MenuCommand::Restore, false, true),
                item(MenuCommand::OpenLog, false, true),
                item(MenuCommand::Quit, false, true),
            ],
        };
        assert_eq!(view.menu, expected);
    }

    #[test]
    fn automatic_shows_its_state_and_sends_the_other() {
        let (view, _) = build(
            &Snapshot {
                automatic: false,
                ..idle()
            },
            NOW,
        );
        assert_eq!(
            view.menu.items[2],
            item(MenuCommand::Automatic(true), false, true)
        );
    }

    /// `holding` is what `wakeshot status` prints, `overridden` in one stable order, so a reordered mirror gives the
    /// same file.
    #[test]
    fn state_file_lists_overridden_keys_sorted_once() {
        let key = |plan, key| OwnedKey { plan, key };
        let snapshot = Snapshot {
            overridden: vec![
                key(POWER_SAVER, SettingKey::LidDc),
                key(BALANCED, SettingKey::SleepDc),
                key(BALANCED, SettingKey::LidAc),
                key(POWER_SAVER, SettingKey::LidDc),
            ],
            ..holding(vec![agent("Claude Code")])
        };
        let (_, state) = build(&snapshot, NOW);
        assert_eq!(
            state,
            StateFile {
                overridden: vec![
                    key(BALANCED, SettingKey::LidAc),
                    key(BALANCED, SettingKey::SleepDc),
                    key(POWER_SAVER, SettingKey::LidDc),
                ],
                ..state_of(
                    &snapshot,
                    vec![text::hold_group(HoldKind::Agent, "Claude Code", 1)]
                )
            }
        );
    }

    #[test]
    fn state_file_round_trips_through_serde_json() {
        let snapshot = Snapshot {
            stepped_back: true,
            requests_on: false,
            overridden: vec![OwnedKey {
                plan: BALANCED,
                key: SettingKey::LidDc,
            }],
            ..holding(vec![
                agent("Claude Code"),
                file(HoldKind::Run, Some("cargo")),
            ])
        };
        let (_, state) = build(&snapshot, NOW);
        let bytes = serde_json::to_vec(&state).unwrap();
        assert_eq!(read::<StateFile>(&bytes), Read::Value(state.clone()));
        assert_eq!(serde_json::from_slice::<StateFile>(&bytes).unwrap(), state);
    }

    /// The CLI writes now plus the length, so a new file's time left gives the length back.
    #[test]
    fn keep_awake_choice_for_a_file_the_tray_did_not_write() {
        let at = |ms: i64| Some(NOW.wall_ms + ms);
        let cases = [
            (None, KeepAwake::UntilOff),
            (at(30 * MINUTE), KeepAwake::Minutes(30)),
            (at(60 * MINUTE - 100), KeepAwake::Minutes(60)),
            (at(60 * MINUTE), KeepAwake::Minutes(60)),
            (at(2 * 60 * MINUTE), KeepAwake::Minutes(120)),
            (at(4 * 60 * MINUTE), KeepAwake::Minutes(240)),
            (at(31 * MINUTE), KeepAwake::Minutes(60)),
            (at(5_000), KeepAwake::Minutes(30)),
            (at(241 * MINUTE), KeepAwake::Minutes(241)),
        ];
        for (expires, choice) in cases {
            assert_eq!(keep_awake_choice(expires, NOW), choice, "{expires:?}");
        }
    }
}
