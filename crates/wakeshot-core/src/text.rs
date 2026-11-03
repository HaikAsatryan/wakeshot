//! Every user-facing string: menu, status line, tooltip pieces, notices (decision 1: English only, no translation
//! layer).
//!
//! Owner: tray track (`p5-text`, then `p5-haik-check`) words them; the signatures are contract, because the core's
//! `present` (Phase 2) calls them. Until `p5-text` is done the bodies are placeholders built from the values, so
//! tests elsewhere must never assert on the wording.

use crate::holds::HoldKind;
use crate::view::{IconState, KeepAwake, MenuCommand, Notice};

/// The longest tooltip Windows shows: `szTip[128]` less the end mark, in UTF-16 units.
pub const TOOLTIP_MAX_UTF16: usize = 127;

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

/// A menu item's text, with `&` before its access key.
pub fn action(command: MenuCommand) -> String {
    format!("{command:?}")
}

/// The Keep awake submenu's own text.
pub fn keep_awake_menu() -> String {
    "Keep awake".to_owned()
}

/// One hold group, such as two Claude Code sessions.
pub fn hold_group(kind: HoldKind, label: &str, count: u32) -> String {
    format!("{label} ({kind:?}, {count})")
}

/// The Keep awake group; `Minutes(n)` is the minutes left.
pub fn keep_awake_group(left: KeepAwake) -> String {
    format!("Keep awake {left:?}")
}

/// The groups cut from a full tooltip, as one more group.
pub fn more_groups(n: u32) -> String {
    format!("and {n} more")
}

/// The menu's status line.
pub fn status_line(status: &Status, groups: &[String]) -> String {
    format!("{:?}: {}", status.icon, groups.join(", "))
}

/// The tooltip; `present` fits it to `TOOLTIP_MAX_UTF16` by dropping whole groups.
pub fn tooltip(status: &Status, groups: &[String]) -> String {
    format!("Wakeshot {:?}: {}", status.icon, groups.join(", "))
}

/// The tooltip of a dry-run tray.
pub fn dry_run(tooltip: &str) -> String {
    format!("{tooltip} (dry run)")
}

pub fn notice(notice: &Notice) -> NoticeText {
    NoticeText {
        title: "Wakeshot".to_owned(),
        body: format!("{notice:?}"),
        warning: false,
        silent: false,
        click: NoticeClick::Nothing,
    }
}
