//! Updates: `ApplyUpdate` only when nothing would be lost, `Quit` once it went through, and the notice after Velopack
//! restarted the tray (decision 42).

use super::Core;
use crate::command::{Command, Failure};
use crate::holds::HoldKind;
use crate::process::Alive;
use crate::view::Notice;

impl Core {
    /// `UpdateReady`: kept until it may run; a newer one replaces it.
    pub(super) fn take_update_ready(&mut self, version: String, out: &mut Vec<Command>) {
        out.push(Command::Log(format!(
            "Update {version} is ready: it applies once nothing holds and nothing is overridden"
        )));
        self.state.update_ready = Some(version);
    }

    /// The settle pass's part: `ApplyUpdate` at once, or as soon as it may run.
    pub(super) fn send_update(&mut self, out: &mut Vec<Command>) {
        if self.state.updating || !self.update_may_run() {
            return;
        }
        let Some(version) = self.state.update_ready.take() else {
            return;
        };
        self.state.updating = true;
        out.push(Command::Log(format!("Applying update {version}")));
        out.push(Command::ApplyUpdate);
    }

    /// Nothing holds, no hold file names a live process (any kind, also with Automatic off: the update would end a
    /// `wakeshot run` or a script's hold), and nothing may be overridden. A Keep awake file always names this tray,
    /// which quits for the update and ends Keep awake anyway (decision 32), so one that no longer counts does not wait;
    /// one that counts holds. A Gone file goes; an Unknown one may be alive. Also not while a countdown or a hibernate
    /// is on: the new tray would not know a closed laptop must still hibernate.
    fn update_may_run(&self) -> bool {
        let state = &self.state;
        let live_file = state
            .files
            .values()
            .any(|f| f.facts.kind != HoldKind::KeepAwake && f.facts.alive != Alive::Gone);
        state.episode.is_none()
            && !live_file
            && !self.may_own()
            && state.work_end.is_none()
            && !state.floor_countdown
            && state.countdown.is_none()
            && !state.hibernating
            && !state.session_ending
    }

    /// `Update(Ok)`: the updater waits for this process to exit, so the tray quits as at the menu's Quit, which also
    /// covers a hold that started since. `Err`: wait for the next `UpdateReady`, which comes only for a newer version
    /// in this run (at most one per version), so the failed one is tried again only after the tray starts again.
    pub(super) fn done_update(&mut self, result: Result<(), Failure>, out: &mut Vec<Command>) {
        self.state.updating = false;
        match result {
            Ok(()) => {
                out.push(Command::Log(
                    "Update applied: Wakeshot quits so the updater can finish".to_owned(),
                ));
                self.menu_quit(out);
            }
            Err(Failure { call, code }) => out.push(Command::Log(format!(
                "Update failed ({call}, code {code}): waiting for the next one"
            ))),
        }
    }

    /// `Updated`: one notice, no sound; it waits while the user is away, like every notice.
    pub(super) fn take_updated(&mut self, version: String, out: &mut Vec<Command>) {
        if !std::mem::replace(&mut self.state.updated_shown, true) {
            self.notify(Notice::Updated { version }, out);
        }
    }
}
