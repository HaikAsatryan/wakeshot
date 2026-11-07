//! The two seams of the ownership routines: plan calls and the safety net, one call per step, so the routine owns
//! the order and a fake can fail or stop any single call. `wakeshot-windows` implements both, real and dry run.

use crate::command::Failure;
use crate::files::JournalFile;
use crate::settings::{Guid, PlanValues, SettingKey};

/// What `journal.json` held when a routine loaded it (restore step 1).
///
/// `Corrupt`: the file exists but does not parse as a `JournalFile`; `Failed`: it could not be read at all, so nothing
/// may be concluded from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalRead {
    Missing,
    Corrupt,
    Failed(Failure),
    Read(JournalFile),
}

/// Power plan calls, one per Windows call.
///
/// `wakeshot-windows` implements it as `Win32Power` and `DryRunPower`. Every method takes `&mut self`: one thread owns
/// it, and fakes keep state even on reads. Callers may rely on:
///
/// - `plans` returns the whole list or an error (a list that ends on any code but 259), never a partial list, so
///   "plan deleted" is judged only from a complete list;
/// - `can_write` is `PowerSettingAccessCheckEx`: an error means policy blocks the write (codes 5 and 1260);
/// - `write` takes only a `SettingKey`, so nothing else can be written, and stores the value only: it is in force
///   only after `set_active` of that plan (`PowerSetActiveScheme`);
/// - `values` reads the active plan id and its five values in one go, for `Event::Plan`; the routines never call it;
/// - `read_default` reads Windows' default of `key` for `plan`, as Windows ships it for the plan's kind (Balanced,
///   High performance, Power saver); only the lost-journal reset calls it.
pub trait PlanIo {
    fn active_plan(&mut self) -> Result<Guid, Failure>;
    fn plans(&mut self) -> Result<Vec<Guid>, Failure>;
    fn can_write(&mut self, key: SettingKey) -> Result<(), Failure>;
    fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure>;
    fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure>;
    fn set_active(&mut self, plan: Guid) -> Result<(), Failure>;
    fn values(&mut self) -> Result<PlanValues, Failure>;

    /// A form that cannot read defaults keeps this answer, so the reset leaves every value as it is. A wrapper around
    /// another `PlanIo` must forward it, or its reset does nothing.
    fn read_default(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
        let _ = (plan, key);
        Err(DEFAULT_NOT_READ)
    }
}

/// `PlanIo::read_default` of a form that does not read defaults. 50 is `ERROR_NOT_SUPPORTED`.
pub const DEFAULT_NOT_READ: Failure = Failure {
    call: "PlanIo::read_default (not supported)",
    code: 50,
};

/// The journal file, the RunOnce value and the guard process, one call per step, so the routine owns their order.
///
/// `wakeshot-windows` implements it as `FileNet`, real and dry run. The journal mutex is not part of it: the callers
/// take it around the whole routine.
pub trait SafetyNet {
    fn load_journal(&mut self) -> JournalRead;
    /// Atomic and write-through: after `Ok` the file is on disk whole.
    fn save_journal(&mut self, journal: &JournalFile) -> Result<(), Failure>;
    fn delete_journal(&mut self) -> Result<(), Failure>;
    /// Moves a corrupt file to `journal.json.corrupt`.
    fn set_aside_journal(&mut self) -> Result<(), Failure>;
    /// `true`: this call added the value.
    fn set_run_once(&mut self) -> Result<bool, Failure>;
    fn clear_run_once(&mut self) -> Result<(), Failure>;
    /// Starts a guard if none runs or the current one is stopping. `true`: this call started it.
    fn ensure_guard(&mut self) -> Result<bool, Failure>;
    /// Sets the current guard's stop event.
    fn stop_guard(&mut self) -> Result<(), Failure>;
}
