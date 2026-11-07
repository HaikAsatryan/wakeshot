//! The tray view and `state.json` through `present`, each sent only when it changed; `state.json` at most once a
//! second, except as `FinishRestoreRequest`.

use super::holds::Found;
use crate::command::Command;
use crate::present::{self, Snapshot};
use crate::settings::OwnedKey;
use crate::state::State;
use crate::time::{Deadline, Now};

/// `state.json` is written at most this often. The first change after a quiet second goes out at once, so
/// `wakeshot status` is never a second behind for a single change; later ones wait for the end of that second.
const STATE_FILE_EVERY_MS: u64 = 1_000;

/// Sends what the user can see when it changed. Gives when a held back `state.json` write is due.
pub(super) fn show(
    state: &mut State,
    found: &Found,
    now: Now,
    out: &mut Vec<Command>,
) -> Option<Deadline> {
    let snapshot = Snapshot {
        automatic: state.automatic,
        holds: found.holders.clone(),
        keep_awake: found.keep_awake,
        screen_on: found.screen,
        requests_on: state.requests.system && !state.requests_failed,
        requests_only: state.requests_only_run
            || state.episode.as_ref().is_some_and(|e| e.requests_only),
        stepped_back: state.stepped_back,
        overridden: state
            .journal
            .iter()
            .map(|e| OwnedKey {
                plan: e.plan,
                key: e.key,
            })
            .collect(),
        tray: state.tray,
    };
    let (view, file) = present::build(&snapshot, now);
    if state.view.as_ref() != Some(&view) {
        out.push(Command::Tray(view.clone()));
        state.view = Some(view);
    }
    // `wakeshot restore` waits for this file to print the result, so it skips the 1 s wait.
    if std::mem::take(&mut state.finish_restore_request) {
        out.push(Command::FinishRestoreRequest(file.clone()));
        state.state_file = Some(file);
        state.state_written_mono = Some(now.mono_ms);
        return None;
    }
    if state.state_file.as_ref() == Some(&file) {
        return None;
    }
    if let Some(at) = state.state_written_mono {
        let due = at.saturating_add(STATE_FILE_EVERY_MS);
        if now.mono_ms < due {
            return Some(Deadline::Mono(due));
        }
    }
    out.push(Command::WriteState(file.clone()));
    state.state_file = Some(file);
    state.state_written_mono = Some(now.mono_ms);
    None
}
