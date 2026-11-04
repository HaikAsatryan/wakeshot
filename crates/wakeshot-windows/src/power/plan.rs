//! Power plan reads and writes: `Win32Power` and `DryRunPower`, the Windows forms of the core's
//! `ownership::io::PlanIo` (architecture section 5, "Power settings"), and, with `test-support`, the throwaway plan
//! copy every real plan test uses (`TestPlan`, `CopyAsActive`; architecture section 9).
//!
//! Owner: power track (`p3-power-plan`).

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use wakeshot_core::command::Failure;
use wakeshot_core::ownership::io::PlanIo;
use wakeshot_core::settings::{Guid, PlanValues, SettingKey};
use windows::Win32::Foundation::{
    ERROR_INVALID_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, HLOCAL, LocalFree, WIN32_ERROR,
};
use windows::Win32::System::Power::{
    ACCESS_AC_POWER_SETTING_INDEX, ACCESS_DC_POWER_SETTING_INDEX, ACCESS_SCHEME, PowerEnumerate,
    PowerGetActiveScheme, PowerReadACValueIndex, PowerReadDCValueIndex, PowerSetActiveScheme,
    PowerSettingAccessCheckEx, PowerWriteACValueIndex, PowerWriteDCValueIndex,
};
use windows::Win32::System::Registry::KEY_WRITE;
use windows::Win32::System::SystemServices::{
    GUID_ENERGY_SAVER_BATTERY_THRESHOLD, GUID_ENERGY_SAVER_SUBGROUP, GUID_LIDCLOSE_ACTION,
    GUID_SLEEP_SUBGROUP, GUID_STANDBY_TIMEOUT, GUID_SYSTEM_BUTTON_SUBGROUP,
};
use windows::core::GUID;

use crate::WinError;
use crate::log::{Level, Log};

const AREA: &str = "power";

const KEYS: [SettingKey; 3] = [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc];

/// A list longer than this is taken as a call that never ends, so a broken `PowerEnumerate` fails the list instead of
/// hanging the IO thread. Windows ships three or four plans; nobody keeps thousands.
const MAX_PLANS: u32 = 4096;

/// A plan value the IO thread reads. Wider than `SettingKey` because `PlanValues` also needs AC sleep and the energy
/// saver threshold, which Wakeshot reads but never writes.
///
/// Owner: power track (Phase 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanSetting {
    LidAc,
    LidDc,
    SleepAc,
    SleepDc,
    SaverThresholdDc,
}

impl From<SettingKey> for PlanSetting {
    fn from(key: SettingKey) -> Self {
        match key {
            SettingKey::LidAc => Self::LidAc,
            SettingKey::LidDc => Self::LidDc,
            SettingKey::SleepDc => Self::SleepDc,
        }
    }
}

impl PlanSetting {
    pub const ALL: [Self; 5] = [
        Self::LidAc,
        Self::LidDc,
        Self::SleepAc,
        Self::SleepDc,
        Self::SaverThresholdDc,
    ];

    /// The name in log lines. The three keys Wakeshot writes use their journal names, so a log line and a journal
    /// entry can be matched by eye.
    pub fn name(self) -> &'static str {
        match self {
            Self::LidAc => "lidAc",
            Self::LidDc => "lidDc",
            Self::SleepAc => "sleepAc",
            Self::SleepDc => "sleepDc",
            Self::SaverThresholdDc => "saverThresholdDc",
        }
    }

    /// The subgroup and setting GUIDs (winnt.h names, from the `windows` crate).
    fn ids(self) -> (GUID, GUID) {
        match self {
            Self::LidAc | Self::LidDc => (GUID_SYSTEM_BUTTON_SUBGROUP, GUID_LIDCLOSE_ACTION),
            Self::SleepAc | Self::SleepDc => (GUID_SLEEP_SUBGROUP, GUID_STANDBY_TIMEOUT),
            Self::SaverThresholdDc => (
                GUID_ENERGY_SAVER_SUBGROUP,
                GUID_ENERGY_SAVER_BATTERY_THRESHOLD,
            ),
        }
    }

    fn on_ac(self) -> bool {
        matches!(self, Self::LidAc | Self::SleepAc)
    }
}

fn win(call: &'static str, rc: u32) -> Result<(), WinError> {
    match rc == ERROR_SUCCESS.0 {
        true => Ok(()),
        false => Err(WinError { call, code: rc }),
    }
}

/// Copies a GUID Windows allocated for the caller, then frees it (Microsoft, `PowerGetActiveScheme` and
/// `PowerDuplicateScheme`: "free this memory" with `LocalFree`).
fn take_guid(call: &'static str, rc: WIN32_ERROR, p: *mut GUID) -> Result<Guid, WinError> {
    if p.is_null() {
        let code = if rc == ERROR_SUCCESS {
            ERROR_INVALID_DATA.0
        } else {
            rc.0
        };
        return Err(WinError { call, code });
    }
    // SAFETY: Windows set `p` to a GUID it allocated with LocalAlloc; it is read once here and freed once below.
    let guid = unsafe { *p };
    // SAFETY: the pointer came from Windows' LocalAlloc and nothing reads it after this.
    unsafe { LocalFree(Some(HLOCAL(p.cast()))) };
    win(call, rc.0)?;
    Ok(Guid(guid.to_u128()))
}

fn get_active() -> Result<Guid, WinError> {
    let mut p: *mut GUID = std::ptr::null_mut();
    // SAFETY: `p` is a live out pointer; the reserved root key is null as the docs require.
    let rc = unsafe { PowerGetActiveScheme(None, &mut p) };
    take_guid("PowerGetActiveScheme", rc, p)
}

/// One `PowerEnumerate` call: the plan at `index`, or the call's code (259 at the end).
fn enumerate_at(index: u32) -> Result<Guid, u32> {
    let mut buf = [0u8; size_of::<GUID>()];
    let mut size = buf.len() as u32;
    // SAFETY: `buf` and `size` live across the call and `size` holds the buffer's length.
    let rc = unsafe {
        PowerEnumerate(
            None,
            None,
            None,
            ACCESS_SCHEME,
            index,
            Some(buf.as_mut_ptr()),
            &mut size,
        )
    };
    if rc != ERROR_SUCCESS {
        return Err(rc.0);
    }
    if size as usize != buf.len() {
        return Err(ERROR_INVALID_DATA.0);
    }
    // SAFETY: the buffer holds exactly one GUID; `read_unaligned` needs no alignment.
    let guid = unsafe { std::ptr::read_unaligned(buf.as_ptr().cast::<GUID>()) };
    Ok(Guid(guid.to_u128()))
}

/// The whole plan list, or an error: only 259 (`ERROR_NO_MORE_ITEMS`) ends it, so a failed call never looks like a
/// shorter list, where "plan deleted" would drop journal entries (architecture section 5, restore step 2).
fn plan_list(mut plan_at: impl FnMut(u32) -> Result<Guid, u32>) -> Result<Vec<Guid>, WinError> {
    const CALL: &str = "PowerEnumerate";
    let mut plans = Vec::new();
    for index in 0..MAX_PLANS {
        match plan_at(index) {
            Ok(plan) => plans.push(plan),
            // Windows always has an active plan, so a list that ends at once is a failed read, not "no plans".
            Err(code) if code == ERROR_NO_MORE_ITEMS.0 && plans.is_empty() => {
                return Err(WinError { call: CALL, code });
            }
            Err(code) if code == ERROR_NO_MORE_ITEMS.0 => return Ok(plans),
            Err(code) => return Err(WinError { call: CALL, code }),
        }
    }
    Err(WinError {
        call: CALL,
        code: ERROR_INVALID_DATA.0,
    })
}

fn read_value(plan: Guid, setting: PlanSetting) -> Result<u32, WinError> {
    let scheme = GUID::from_u128(plan.0);
    let (sub, id) = setting.ids();
    let mut value = 0u32;
    if setting.on_ac() {
        // SAFETY: the GUIDs and the out value live across the call.
        let rc = unsafe {
            PowerReadACValueIndex(None, Some(&scheme), Some(&sub), Some(&id), &mut value)
        };
        win("PowerReadACValueIndex", rc.0)?;
    } else {
        // SAFETY: as above.
        let rc = unsafe {
            PowerReadDCValueIndex(None, Some(&scheme), Some(&sub), Some(&id), &mut value)
        };
        win("PowerReadDCValueIndex", rc)?;
    }
    Ok(value)
}

/// Takes only a `SettingKey`, so nothing but the three keys can be written (decision 17).
fn write_value(plan: Guid, key: SettingKey, value: u32) -> Result<(), WinError> {
    let setting = PlanSetting::from(key);
    let scheme = GUID::from_u128(plan.0);
    let (sub, id) = setting.ids();
    if setting.on_ac() {
        // SAFETY: the GUIDs live across the call.
        let rc = unsafe { PowerWriteACValueIndex(None, &scheme, Some(&sub), Some(&id), value) };
        win("PowerWriteACValueIndex", rc.0)
    } else {
        // SAFETY: as above.
        let rc = unsafe { PowerWriteDCValueIndex(None, &scheme, Some(&sub), Some(&id), value) };
        win("PowerWriteDCValueIndex", rc)
    }
}

fn set_active_plan(plan: Guid) -> Result<(), WinError> {
    let scheme = GUID::from_u128(plan.0);
    // SAFETY: the GUID lives across the call.
    let rc = unsafe { PowerSetActiveScheme(None, Some(&scheme)) };
    win("PowerSetActiveScheme", rc.0)
}

fn access_check(key: SettingKey) -> Result<(), WinError> {
    let setting = PlanSetting::from(key);
    let (_, id) = setting.ids();
    let accessor = match setting.on_ac() {
        true => ACCESS_AC_POWER_SETTING_INDEX,
        false => ACCESS_DC_POWER_SETTING_INDEX,
    };
    // SAFETY: the GUID lives across the call.
    let rc = unsafe { PowerSettingAccessCheckEx(accessor, Some(&id), KEY_WRITE) };
    win("PowerSettingAccessCheckEx", rc.0)
}

fn key_value(values: &PlanValues, key: SettingKey) -> u32 {
    match key {
        SettingKey::LidAc => values.lid_ac,
        SettingKey::LidDc => values.lid_dc,
        SettingKey::SleepDc => values.sleep_dc,
    }
}

fn key_value_mut(values: &mut PlanValues, key: SettingKey) -> &mut u32 {
    match key {
        SettingKey::LidAc => &mut values.lid_ac,
        SettingKey::LidDc => &mut values.lid_dc,
        SettingKey::SleepDc => &mut values.sleep_dc,
    }
}

/// The stored value last read or written for each plan and key, so a write line can name the old value without a
/// read of its own. The routines always read before they write, so it is the value the routine decided on.
#[derive(Debug, Default)]
struct LastSeen(BTreeMap<(Guid, SettingKey), u32>);

impl LastSeen {
    fn note(&mut self, plan: Guid, key: SettingKey, value: u32) {
        self.0.insert((plan, key), value);
    }

    fn get(&self, plan: Guid, key: SettingKey) -> Option<u32> {
        self.0.get(&(plan, key)).copied()
    }
}

/// Logs one write with its old and new value (architecture section 8) and passes its result on.
fn log_write(
    log: &dyn Log,
    seen: &mut LastSeen,
    dry_run: bool,
    plan: Guid,
    key: SettingKey,
    value: u32,
    result: Result<(), Failure>,
) -> Result<(), Failure> {
    let old = seen
        .get(plan, key)
        .map_or_else(|| "not read".to_owned(), |v| v.to_string());
    let what = format!(
        "{} on plan {plan}: {old} to {value}",
        PlanSetting::from(key).name()
    );
    let verb = match dry_run {
        true => "dry run: would write",
        false => "wrote",
    };
    match result {
        Ok(()) => {
            log.line(Level::Info, AREA, &format!("{verb} {what}"));
            seen.note(plan, key, value);
        }
        Err(f) => {
            let verb = match dry_run {
                true => verb,
                false => "write",
            };
            let text = format!("{verb} {what} failed: {} code {}", f.call, f.code);
            log.line(Level::Warn, AREA, &text);
        }
    }
    result
}

fn log_set_active(
    log: &dyn Log,
    dry_run: bool,
    plan: Guid,
    result: Result<(), Failure>,
) -> Result<(), Failure> {
    let verb = match dry_run {
        true => "dry run: would set",
        false => "set",
    };
    match result {
        Ok(()) => log.line(Level::Info, AREA, &format!("{verb} active plan {plan}")),
        Err(f) => {
            let text = format!(
                "{verb} active plan {plan} failed: {} code {}",
                f.call, f.code
            );
            log.line(Level::Warn, AREA, &text);
        }
    }
    result
}

/// The real plan calls. One thread owns it (the IO thread, the guard, restore mode, a hook or the CLI).
pub struct Win32Power {
    log: Arc<dyn Log>,
    seen: LastSeen,
}

impl Win32Power {
    pub fn new(log: Arc<dyn Log>) -> Self {
        Self {
            log,
            seen: LastSeen::default(),
        }
    }

    /// The five values of one plan, each read from that plan.
    fn values_of(&mut self, plan: Guid) -> Result<PlanValues, Failure> {
        let values = PlanValues {
            active: plan,
            lid_ac: read_value(plan, PlanSetting::LidAc)?,
            lid_dc: read_value(plan, PlanSetting::LidDc)?,
            sleep_ac: read_value(plan, PlanSetting::SleepAc)?,
            sleep_dc: read_value(plan, PlanSetting::SleepDc)?,
            saver_threshold_dc: read_value(plan, PlanSetting::SaverThresholdDc)?,
        };
        for key in KEYS {
            self.seen.note(plan, key, key_value(&values, key));
        }
        Ok(values)
    }
}

impl fmt::Debug for Win32Power {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Win32Power")
            .field("seen", &self.seen)
            .finish_non_exhaustive()
    }
}

impl PlanIo for Win32Power {
    fn active_plan(&mut self) -> Result<Guid, Failure> {
        Ok(get_active()?)
    }

    fn plans(&mut self) -> Result<Vec<Guid>, Failure> {
        Ok(plan_list(enumerate_at)?)
    }

    fn can_write(&mut self, key: SettingKey) -> Result<(), Failure> {
        Ok(access_check(key)?)
    }

    fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
        let value = read_value(plan, key.into())?;
        self.seen.note(plan, key, value);
        Ok(value)
    }

    fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure> {
        let result = write_value(plan, key, value).map_err(Failure::from);
        log_write(&*self.log, &mut self.seen, false, plan, key, value, result)
    }

    fn set_active(&mut self, plan: Guid) -> Result<(), Failure> {
        let result = set_active_plan(plan).map_err(Failure::from);
        log_set_active(&*self.log, false, plan, result)
    }

    fn values(&mut self) -> Result<PlanValues, Failure> {
        let active = get_active()?;
        self.values_of(active)
    }
}

/// A dry-run value for one plan and key, and the real stored value it was written over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Overlaid {
    base: u32,
    value: u32,
}

/// Reads from `P`, keeps writes in memory, and never calls `P::write` or `P::set_active` (track note 8). Reads
/// return the dry-run value while the real stored value is still the one it was written over; once that changes
/// (the user in Settings), the real value wins and the dry-run value is dropped, as a real write would be
/// overwritten. Each write makes one read of `P` for that real value, so a script on `P` counts it.
///
/// `set_active` is logged and recorded, and `active_plan` still answers from `P`: the routines call `set_active` only
/// with the plan `active_plan` just returned, and a real plan switch must stay visible. Change it when a caller
/// sets a plan active that is not the active one.
pub struct DryRunPower<P> {
    inner: P,
    log: Arc<dyn Log>,
    overlay: BTreeMap<(Guid, SettingKey), Overlaid>,
    seen: LastSeen,
    set_active_calls: Vec<Guid>,
}

impl<P: PlanIo> DryRunPower<P> {
    pub fn new(inner: P, log: Arc<dyn Log>) -> Self {
        Self {
            inner,
            log,
            overlay: BTreeMap::new(),
            seen: LastSeen::default(),
            set_active_calls: Vec::new(),
        }
    }

    /// Every `set_active` so far, in order.
    pub fn set_active_calls(&self) -> &[Guid] {
        &self.set_active_calls
    }

    /// The value a read returns, given the real stored value.
    fn view(&mut self, plan: Guid, key: SettingKey, real: u32) -> u32 {
        match self.overlay.get(&(plan, key)) {
            None => real,
            Some(o) if o.base == real => o.value,
            Some(o) => {
                let text = format!(
                    "dry run: {} on plan {plan} changed outside Wakeshot to {real}; dropped the dry-run value {}",
                    PlanSetting::from(key).name(),
                    o.value
                );
                self.log.line(Level::Info, AREA, &text);
                self.overlay.remove(&(plan, key));
                real
            }
        }
    }
}

impl<P: fmt::Debug> fmt::Debug for DryRunPower<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DryRunPower")
            .field("inner", &self.inner)
            .field("overlay", &self.overlay)
            .field("seen", &self.seen)
            .field("set_active_calls", &self.set_active_calls)
            .finish_non_exhaustive()
    }
}

impl<P: PlanIo> PlanIo for DryRunPower<P> {
    fn active_plan(&mut self) -> Result<Guid, Failure> {
        self.inner.active_plan()
    }

    fn plans(&mut self) -> Result<Vec<Guid>, Failure> {
        self.inner.plans()
    }

    fn can_write(&mut self, key: SettingKey) -> Result<(), Failure> {
        self.inner.can_write(key)
    }

    fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
        let real = self.inner.read(plan, key)?;
        let value = self.view(plan, key, real);
        self.seen.note(plan, key, value);
        Ok(value)
    }

    fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure> {
        let result = self.inner.read(plan, key).map(|base| {
            if value == base {
                self.overlay.remove(&(plan, key));
            } else {
                self.overlay.insert((plan, key), Overlaid { base, value });
            }
        });
        log_write(&*self.log, &mut self.seen, true, plan, key, value, result)
    }

    fn set_active(&mut self, plan: Guid) -> Result<(), Failure> {
        self.set_active_calls.push(plan);
        log_set_active(&*self.log, true, plan, Ok(()))
    }

    fn values(&mut self) -> Result<PlanValues, Failure> {
        let mut values = self.inner.values()?;
        for key in KEYS {
            let shown = self.view(values.active, key, key_value(&values, key));
            *key_value_mut(&mut values, key) = shown;
            self.seen.note(values.active, key, shown);
        }
        Ok(values)
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use copies::{CopyAsActive, TestPlan, ValuesOf};

/// The throwaway plan copy and the wrapper that treats it as active (architecture section 9). Test builds only: no
/// exe ships code that creates or deletes a plan.
#[cfg(any(test, feature = "test-support"))]
mod copies {
    use wakeshot_core::process::Lookup;
    use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_MORE_DATA};
    use windows::Win32::System::Power::{
        PowerDeleteScheme, PowerDuplicateScheme, PowerReadFriendlyName, PowerWriteFriendlyName,
    };

    use super::*;
    use crate::process::{Liveness, SystemLiveness};

    const NAME_PREFIX: &str = "Wakeshot test ";

    /// A copy of the active plan named "Wakeshot test <pid>" with this process's pid, deleted on drop. Nothing here
    /// calls `PowerSetActiveScheme`, so the copy is never active and its writes never reach the user's plan.
    #[derive(Debug)]
    pub struct TestPlan {
        plan: Guid,
    }

    impl TestPlan {
        /// The name of a copy made by the process `pid`.
        pub fn name_for(pid: u32) -> String {
            format!("{NAME_PREFIX}{pid}")
        }

        pub fn new() -> Result<Self, WinError> {
            let source = GUID::from_u128(get_active()?.0);
            let mut p: *mut GUID = std::ptr::null_mut();
            // SAFETY: a null `p` asks Windows to allocate the new GUID (Microsoft, `PowerDuplicateScheme`);
            // `take_guid` frees it.
            let rc = unsafe { PowerDuplicateScheme(None, &source, &mut p) };
            // Built before the rename, so a failed rename deletes the copy on the way out.
            let copy = Self {
                plan: take_guid("PowerDuplicateScheme", rc, p)?,
            };
            write_friendly_name(copy.plan, &Self::name_for(std::process::id()))?;
            Ok(copy)
        }

        pub fn guid(&self) -> Guid {
            self.plan
        }

        /// Deletes the copies earlier test runs left: named "Wakeshot test <pid>", that pid not running, and not
        /// the active plan. Returns the deleted ones. Only `real_` tests and the harness's `power_check cleanup`
        /// call it, and both run one at a time across all cards (review M7: a sweep must never delete a copy
        /// another card is walking). A pid now reused by another process keeps its copy until that process ends,
        /// which errs on the side of keeping.
        pub fn sweep() -> Result<Vec<Guid>, WinError> {
            let active = get_active()?;
            let mut named = Vec::new();
            for plan in plan_list(enumerate_at)? {
                // A plan whose name cannot be read is not known to be a test copy, so it stays.
                if let Ok(name) = friendly_name(plan) {
                    named.push((plan, name));
                }
            }
            let liveness = SystemLiveness;
            let leftovers = pick_leftovers(&named, active, std::process::id(), |pid| {
                liveness.start_of_pid(pid)
            });
            for &plan in &leftovers {
                delete_plan(plan)?;
            }
            Ok(leftovers)
        }
    }

    impl Drop for TestPlan {
        fn drop(&mut self) {
            // Windows refuses to delete the active plan (research, "Plan copy"); say so instead of failing later.
            if get_active() == Ok(self.plan) {
                eprintln!("TestPlan: {} is the active plan, so it is kept", self.plan);
                return;
            }
            if let Err(e) = delete_plan(self.plan) {
                eprintln!(
                    "TestPlan: deleting {} failed: {} code {}",
                    self.plan, e.call, e.code
                );
            }
        }
    }

    /// The pid in a test copy's name, only for the exact form "Wakeshot test <digits>": "Wakeshot test e2e" and any
    /// other name are never a sweep's.
    pub(super) fn test_pid(name: &str) -> Option<u32> {
        let digits = name.strip_prefix(NAME_PREFIX)?;
        match !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            true => digits.parse().ok(),
            false => None,
        }
    }

    pub(super) fn pick_leftovers(
        named: &[(Guid, String)],
        active: Guid,
        me: u32,
        lookup: impl Fn(u32) -> Lookup,
    ) -> Vec<Guid> {
        named
            .iter()
            .filter_map(|(plan, name)| {
                let pid = test_pid(name)?;
                let dead = *plan != active && pid != me && lookup(pid) == Lookup::NotRunning;
                dead.then_some(*plan)
            })
            .collect()
    }

    pub(super) fn delete_plan(plan: Guid) -> Result<(), WinError> {
        let scheme = GUID::from_u128(plan.0);
        // SAFETY: the GUID lives across the call.
        let rc = unsafe { PowerDeleteScheme(None, &scheme) };
        win("PowerDeleteScheme", rc.0)
    }

    fn write_friendly_name(plan: Guid, name: &str) -> Result<(), WinError> {
        let scheme = GUID::from_u128(plan.0);
        let bytes: Vec<u8> = name
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        // SAFETY: the GUID and the NUL-terminated UTF-16 bytes live across the call.
        let rc = unsafe { PowerWriteFriendlyName(None, &scheme, None, None, &bytes) };
        win("PowerWriteFriendlyName", rc.0)
    }

    pub(super) fn friendly_name(plan: Guid) -> Result<String, WinError> {
        const CALL: &str = "PowerReadFriendlyName";
        let scheme = GUID::from_u128(plan.0);
        let mut size = 0u32;
        // SAFETY: a null buffer asks only for the size, written to `size`.
        let rc = unsafe { PowerReadFriendlyName(None, Some(&scheme), None, None, None, &mut size) };
        if rc != ERROR_SUCCESS && rc != ERROR_MORE_DATA {
            return Err(WinError {
                call: CALL,
                code: rc.0,
            });
        }
        let mut buf = vec![0u8; size as usize];
        // SAFETY: `buf` holds `size` bytes and both live across the call.
        let rc = unsafe {
            PowerReadFriendlyName(
                None,
                Some(&scheme),
                None,
                None,
                Some(buf.as_mut_ptr()),
                &mut size,
            )
        };
        win(CALL, rc.0)?;
        let wide: Vec<u16> = buf
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| u16::from_le_bytes(c))
            .take_while(|&c| c != 0)
            .collect();
        Ok(String::from_utf16_lossy(&wide))
    }

    /// The five values of any plan, which `PlanIo` gives only for the active one, so `CopyAsActive` can answer
    /// `values()` for its copy.
    pub trait ValuesOf {
        fn values_of(&mut self, plan: Guid) -> Result<PlanValues, Failure>;
    }

    impl ValuesOf for Win32Power {
        fn values_of(&mut self, plan: Guid) -> Result<PlanValues, Failure> {
            Win32Power::values_of(self, plan)
        }
    }

    /// A `PlanIo` that reports a plan copy as the active plan, so the core routines run against the copy while the
    /// user's plan stays active. `set_active` of the copy is recorded and never reaches Windows; "in force" is the
    /// copy's stored values at the last recorded `set_active` (at creation before the first). A write or
    /// `set_active` for any other plan is refused before Windows sees it, so a journal entry for the user's plan can
    /// never be written through it.
    #[derive(Debug)]
    pub struct CopyAsActive<P = Win32Power> {
        inner: P,
        copy: Guid,
        in_force: [u32; 3],
        set_active_count: usize,
    }

    impl CopyAsActive<Win32Power> {
        pub fn over(copy: &TestPlan, power: Win32Power) -> Result<Self, Failure> {
            Self::new(power, copy.guid())
        }
    }

    impl<P: PlanIo + ValuesOf> CopyAsActive<P> {
        pub(super) fn new(mut inner: P, copy: Guid) -> Result<Self, Failure> {
            let in_force = stored_keys(&mut inner, copy)?;
            Ok(Self {
                inner,
                copy,
                in_force,
                set_active_count: 0,
            })
        }

        pub fn copy(&self) -> Guid {
            self.copy
        }

        /// The copy's value for `key` as of the last recorded `set_active`.
        pub fn in_force(&self, key: SettingKey) -> u32 {
            self.in_force[key_index(key)]
        }

        pub fn set_active_count(&self) -> usize {
            self.set_active_count
        }

        fn only_the_copy(&self, plan: Guid, call: &'static str) -> Result<(), Failure> {
            match plan == self.copy {
                true => Ok(()),
                false => Err(Failure {
                    call,
                    code: ERROR_ACCESS_DENIED.0,
                }),
            }
        }
    }

    fn key_index(key: SettingKey) -> usize {
        match key {
            SettingKey::LidAc => 0,
            SettingKey::LidDc => 1,
            SettingKey::SleepDc => 2,
        }
    }

    fn stored_keys(inner: &mut impl PlanIo, plan: Guid) -> Result<[u32; 3], Failure> {
        Ok([
            inner.read(plan, SettingKey::LidAc)?,
            inner.read(plan, SettingKey::LidDc)?,
            inner.read(plan, SettingKey::SleepDc)?,
        ])
    }

    impl<P: PlanIo + ValuesOf> PlanIo for CopyAsActive<P> {
        fn active_plan(&mut self) -> Result<Guid, Failure> {
            Ok(self.copy)
        }

        fn plans(&mut self) -> Result<Vec<Guid>, Failure> {
            self.inner.plans()
        }

        fn can_write(&mut self, key: SettingKey) -> Result<(), Failure> {
            self.inner.can_write(key)
        }

        fn read(&mut self, plan: Guid, key: SettingKey) -> Result<u32, Failure> {
            self.inner.read(plan, key)
        }

        fn write(&mut self, plan: Guid, key: SettingKey, value: u32) -> Result<(), Failure> {
            self.only_the_copy(plan, "CopyAsActive::write")?;
            self.inner.write(plan, key, value)
        }

        fn set_active(&mut self, plan: Guid) -> Result<(), Failure> {
            self.only_the_copy(plan, "CopyAsActive::set_active")?;
            self.in_force = stored_keys(&mut self.inner, plan)?;
            self.set_active_count += 1;
            Ok(())
        }

        fn values(&mut self) -> Result<PlanValues, Failure> {
            self.inner.values_of(self.copy)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use wakeshot_core::ownership::fake::{Action, Call, FAKE_MISSING, FakePlans};
    use wakeshot_core::process::{FileTime, Lookup};

    use super::copies::{friendly_name, pick_leftovers, test_pid};
    use super::*;
    use crate::log::MemoryLog;

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const OTHER: Guid = Guid(0xa000_0000_0000_0000_0000_0000_0000_0001);
    const COPY: Guid = Guid(0xa000_0000_0000_0000_0000_0000_0000_0002);

    fn user_values(plan: Guid) -> PlanValues {
        PlanValues {
            active: plan,
            lid_ac: 1,
            lid_dc: 1,
            sleep_ac: 10_800,
            sleep_dc: 600,
            saver_threshold_dc: 30,
        }
    }

    fn lines(log: &MemoryLog) -> Vec<String> {
        log.lines
            .lock()
            .unwrap()
            .iter()
            .map(|(_, _, text)| text.clone())
            .collect()
    }

    impl ValuesOf for FakePlans {
        fn values_of(&mut self, plan: Guid) -> Result<PlanValues, Failure> {
            let state = self.state();
            state.plans.get(&plan).map(|p| p.stored).ok_or(Failure {
                call: "FakePlans::values_of",
                code: FAKE_MISSING,
            })
        }
    }

    #[test]
    fn setting_guids_match_the_spike() {
        // The spike's literals (`spikes/power-settings/src/power.rs`), which its write and copy tests used.
        const SUB_BUTTONS: u128 = 0x4f971e89_eebd_4455_a8de_9e59040e7347;
        const LIDACTION: u128 = 0x5ca83367_6e45_459f_a27b_476b1d01c936;
        const SUB_SLEEP: u128 = 0x238c9fa8_0aad_41ed_83f4_97be242c8f20;
        const STANDBYIDLE: u128 = 0x29f6c1db_86da_48c5_9fdb_f2b67b1f44da;
        const SUB_ENERGYSAVER: u128 = 0xde830923_a562_41af_a086_e3a2c6bad2da;
        const ESBATTTHRESHOLD: u128 = 0xe69653ca_cf7f_4f05_aa73_cb833fa90ad4;
        let spike = [
            (PlanSetting::LidAc, SUB_BUTTONS, LIDACTION, true),
            (PlanSetting::LidDc, SUB_BUTTONS, LIDACTION, false),
            (PlanSetting::SleepAc, SUB_SLEEP, STANDBYIDLE, true),
            (PlanSetting::SleepDc, SUB_SLEEP, STANDBYIDLE, false),
            (
                PlanSetting::SaverThresholdDc,
                SUB_ENERGYSAVER,
                ESBATTTHRESHOLD,
                false,
            ),
        ];
        for (setting, sub, id, ac) in spike {
            let (s, i) = setting.ids();
            assert_eq!((s.to_u128(), i.to_u128(), setting.on_ac()), (sub, id, ac));
        }
        assert_eq!(spike.map(|(s, ..)| s), PlanSetting::ALL);
        let keys = KEYS.map(PlanSetting::from);
        assert_eq!(
            keys,
            [PlanSetting::LidAc, PlanSetting::LidDc, PlanSetting::SleepDc]
        );
        assert_eq!(keys.map(PlanSetting::name), ["lidAc", "lidDc", "sleepDc"]);
    }

    #[test]
    fn plan_list_ends_only_at_259() {
        fn scripted(calls: Vec<Result<Guid, u32>>) -> impl FnMut(u32) -> Result<Guid, u32> {
            // An index past the script panics: the loop went on after the list had ended.
            move |index| calls[index as usize]
        }
        let failed = |code| -> Result<Vec<Guid>, WinError> {
            Err(WinError {
                call: "PowerEnumerate",
                code,
            })
        };
        let end = Err(ERROR_NO_MORE_ITEMS.0);

        assert_eq!(
            plan_list(scripted(vec![Ok(BALANCED), Ok(OTHER), end])),
            Ok(vec![BALANCED, OTHER])
        );
        assert_eq!(
            plan_list(scripted(vec![Ok(BALANCED), Err(5), Ok(OTHER), end])),
            failed(5),
            "5 is an error, never a short list"
        );
        assert_eq!(
            plan_list(scripted(vec![Ok(BALANCED), Err(234)])),
            failed(234)
        );
        assert_eq!(
            plan_list(scripted(vec![end])),
            failed(259),
            "no plan at all"
        );
        assert_eq!(
            plan_list(|_| Ok(BALANCED)),
            failed(ERROR_INVALID_DATA.0),
            "a list that never ends"
        );
    }

    #[test]
    fn dry_run_never_writes_inner() {
        let fake = FakePlans::new(user_values(BALANCED));
        fake.add_plan(user_values(OTHER));
        let mut dry = DryRunPower::new(fake.clone(), Arc::new(MemoryLog::default()));

        // An apply's calls, a plan read, then a restore's, as the routines make them.
        let plan = dry.active_plan().unwrap();
        assert_eq!(dry.plans().unwrap(), [BALANCED, OTHER]);
        for key in KEYS {
            dry.can_write(key).unwrap();
            dry.read(plan, key).unwrap();
            dry.write(plan, key, 0).unwrap();
        }
        dry.set_active(plan).unwrap();
        dry.values().unwrap();
        for key in KEYS {
            dry.write(plan, key, key_value(&user_values(plan), key))
                .unwrap();
        }
        dry.write(OTHER, SettingKey::LidDc, 0).unwrap();
        dry.set_active(plan).unwrap();

        let state = fake.state();
        let changed: Vec<&Call> = state
            .calls
            .iter()
            .filter(|c| matches!(c, Call::Write(..) | Call::SetActive(..)))
            .collect();
        assert!(changed.is_empty(), "{changed:?}");
        for plan in [BALANCED, OTHER] {
            assert_eq!(state.plans[&plan].stored, user_values(plan));
            assert_eq!(state.plans[&plan].in_force, user_values(plan));
        }
        assert_eq!(state.active, BALANCED);
        assert_eq!(dry.set_active_calls(), [BALANCED, BALANCED]);
    }

    #[test]
    fn dry_run_reads_its_own_writes() {
        let fake = FakePlans::new(user_values(BALANCED));
        fake.add_plan(user_values(OTHER));
        let log = Arc::new(MemoryLog::default());
        let mut dry = DryRunPower::new(fake.clone(), log.clone());

        dry.write(BALANCED, SettingKey::LidDc, 0).unwrap();
        dry.write(BALANCED, SettingKey::SleepDc, 0).unwrap();
        assert_eq!(dry.read(BALANCED, SettingKey::LidDc), Ok(0));
        assert_eq!(dry.read(OTHER, SettingKey::LidDc), Ok(1), "only that plan");
        let values = dry.values().unwrap();
        assert_eq!(
            values,
            PlanValues {
                lid_dc: 0,
                sleep_dc: 0,
                ..user_values(BALANCED)
            }
        );
        assert_eq!(fake.state().stored(BALANCED, SettingKey::LidDc), Some(1));

        // Writing the real value back leaves nothing in memory.
        dry.write(BALANCED, SettingKey::LidDc, 1).unwrap();
        assert_eq!(dry.read(BALANCED, SettingKey::LidDc), Ok(1));
        assert!(!dry.overlay.contains_key(&(BALANCED, SettingKey::LidDc)));

        // The user changes the real value: it wins over the dry-run value, as over a real write.
        fake.state().user_write(BALANCED, SettingKey::SleepDc, 900);
        assert_eq!(dry.read(BALANCED, SettingKey::SleepDc), Ok(900));
        assert_eq!(dry.values().unwrap().sleep_dc, 900);
        assert!(dry.overlay.is_empty());
        assert!(
            lines(&log)
                .iter()
                .any(|l| l.contains("sleepDc on plan")
                    && l.contains("changed outside Wakeshot to 900")),
            "{:?}",
            lines(&log)
        );

        // The dry run's own read of the real value fails: so does the write, and nothing is kept.
        let next = fake.state().calls.len();
        fake.script(next, Action::Fail(31));
        let failed = dry.write(BALANCED, SettingKey::LidAc, 0).unwrap_err();
        assert_eq!(failed.code, 31);
        assert_eq!(dry.read(BALANCED, SettingKey::LidAc), Ok(1));
    }

    #[test]
    fn write_log_has_old_and_new() {
        // The helper `Win32Power::write` logs through, with a success and a failure.
        let log = MemoryLog::default();
        let mut seen = LastSeen::default();
        let denied = Failure {
            call: "PowerWriteDCValueIndex",
            code: 5,
        };
        let key = SettingKey::LidDc;
        log_write(&log, &mut seen, false, BALANCED, key, 0, Ok(())).unwrap();
        seen.note(BALANCED, key, 1);
        log_write(&log, &mut seen, false, BALANCED, key, 0, Ok(())).unwrap();
        log_write(&log, &mut seen, false, BALANCED, key, 3, Err(denied)).unwrap_err();
        assert_eq!(
            seen.get(BALANCED, key),
            Some(0),
            "a failed write is not seen"
        );
        let plan = "381b4222-f694-41f0-9685-ff5bb260df2e";
        assert_eq!(
            lines(&log),
            [
                format!("wrote lidDc on plan {plan}: not read to 0"),
                format!("wrote lidDc on plan {plan}: 1 to 0"),
                format!("write lidDc on plan {plan}: 0 to 3 failed: PowerWriteDCValueIndex code 5"),
            ]
        );
        assert_eq!(log.lines.lock().unwrap()[2].0, Level::Warn);

        // The dry run: the value its last read returned, then its own last write.
        let fake = FakePlans::new(user_values(BALANCED));
        let log = Arc::new(MemoryLog::default());
        let mut dry = DryRunPower::new(fake, log.clone());
        assert_eq!(dry.read(BALANCED, SettingKey::SleepDc), Ok(600));
        dry.write(BALANCED, SettingKey::SleepDc, 0).unwrap();
        dry.write(BALANCED, SettingKey::SleepDc, 600).unwrap();
        dry.set_active(BALANCED).unwrap();
        assert_eq!(
            lines(&log),
            [
                format!("dry run: would write sleepDc on plan {plan}: 600 to 0"),
                format!("dry run: would write sleepDc on plan {plan}: 0 to 600"),
                format!("dry run: would set active plan {plan}"),
            ]
        );
    }

    #[test]
    fn copy_as_active_reports_the_copy_and_keeps_in_force() {
        let fake = FakePlans::new(user_values(BALANCED));
        fake.add_plan(user_values(COPY));
        let mut io = CopyAsActive::new(fake.clone(), COPY).unwrap();
        assert_eq!(io.active_plan(), Ok(COPY));
        assert_eq!(io.values().unwrap(), user_values(COPY));

        io.write(COPY, SettingKey::LidDc, 0).unwrap();
        assert_eq!(
            io.in_force(SettingKey::LidDc),
            1,
            "stored, not in force yet"
        );
        assert_eq!(io.values().unwrap().lid_dc, 0);
        io.set_active(COPY).unwrap();
        assert_eq!(io.in_force(SettingKey::LidDc), 0);
        assert_eq!(io.in_force(SettingKey::SleepDc), 600);
        assert_eq!(io.set_active_count(), 1);

        // A set_active whose read of the copy fails records nothing.
        io.write(COPY, SettingKey::SleepDc, 0).unwrap();
        let next = fake.state().calls.len();
        fake.script(next, Action::Fail(21));
        assert_eq!(io.set_active(COPY).unwrap_err().code, 21);
        assert_eq!(
            (io.in_force(SettingKey::SleepDc), io.set_active_count()),
            (600, 1)
        );

        let state = fake.state();
        assert!(
            !state
                .calls
                .iter()
                .any(|c| matches!(c, Call::SetActive(_) | Call::ActivePlan))
        );
        assert_eq!(state.active, BALANCED);
        assert_eq!(state.plans[&BALANCED].stored, user_values(BALANCED));
    }

    #[test]
    fn copy_as_active_refuses_other_plans() {
        let fake = FakePlans::new(user_values(BALANCED));
        fake.add_plan(user_values(COPY));
        let mut io = CopyAsActive::new(fake.clone(), COPY).unwrap();
        let calls_before = fake.state().calls.len();

        let refused = io.write(BALANCED, SettingKey::LidDc, 0).unwrap_err();
        assert_eq!(
            refused,
            Failure {
                call: "CopyAsActive::write",
                code: 5
            }
        );
        let refused = io.set_active(BALANCED).unwrap_err();
        assert_eq!(refused.call, "CopyAsActive::set_active");
        assert_eq!(io.set_active_count(), 0);
        assert_eq!(
            fake.state().calls.len(),
            calls_before,
            "nothing reached the plans"
        );
        assert_eq!(fake.state().plans[&BALANCED].stored, user_values(BALANCED));
        // Reads of other plans are harmless and pass through.
        assert_eq!(io.read(BALANCED, SettingKey::SleepDc), Ok(600));
    }

    #[test]
    fn test_pid_takes_only_digit_names() {
        assert_eq!(test_pid("Wakeshot test 1234"), Some(1234));
        assert_eq!(TestPlan::name_for(77), "Wakeshot test 77");
        assert_eq!(test_pid(&TestPlan::name_for(77)), Some(77));
        for name in [
            "Wakeshot test e2e",
            "Wakeshot test ",
            "Wakeshot test +12",
            "Wakeshot test 12 ",
            "Wakeshot test 99999999999",
            "wakeshot test 12",
            "Balanced",
        ] {
            assert_eq!(test_pid(name), None, "{name}");
        }
    }

    #[test]
    fn sweep_picks_only_dead_inactive_test_copies() {
        let plan = |n: u128| Guid(0xb000_0000_0000_0000_0000_0000_0000_0000 | n);
        let me = 4242;
        let named: Vec<(Guid, String)> = [
            (BALANCED, "Balanced"),
            (plan(1), "Wakeshot test e2e"),
            (plan(2), "Wakeshot test 4242"),
            (plan(3), "Wakeshot test 100"),
            (plan(4), "Wakeshot test 200"),
            (plan(5), "Wakeshot test 300"),
            (plan(6), "Wakeshot test 100"),
        ]
        .into_iter()
        .map(|(g, n)| (g, n.to_owned()))
        .collect();
        let lookup = |pid| match pid {
            100 => Lookup::NotRunning,
            200 => Lookup::Running(FileTime(1)),
            _ => Lookup::Unknown,
        };
        // plan(6) is dead but active; 4242 is this process; 200 runs; 300 is not known.
        assert_eq!(pick_leftovers(&named, plan(6), me, lookup), [plan(3)]);
        assert!(pick_leftovers(&named, plan(3), 100, lookup).is_empty());
    }

    /// Wakeshot's own target for a key, or another value when the plan already has it, so every write shows.
    fn visible_target(key: SettingKey, old: u32) -> u32 {
        match (key, old) {
            (_, v) if v != 0 => 0,
            (SettingKey::SleepDc, _) => 600,
            _ => 1,
        }
    }

    /// `powercfg /qh` for one value, a second reader beside ours. English output only (the test laptop).
    fn powercfg_index(plan: Guid, setting: PlanSetting) -> u32 {
        let (sub, id) = setting.ids();
        let (sub, id) = (Guid(sub.to_u128()), Guid(id.to_u128()));
        let out = Command::new("powercfg")
            .args([
                "/qh".to_owned(),
                plan.to_string(),
                sub.to_string(),
                id.to_string(),
            ])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let label = match setting.on_ac() {
            true => "Current AC Power Setting Index:",
            false => "Current DC Power Setting Index:",
        };
        let line = text
            .lines()
            .find(|l| l.contains(label))
            .unwrap_or_else(|| panic!("no {label} in powercfg output:\n{text}"));
        let hex = line.rsplit("0x").next().unwrap().trim();
        u32::from_str_radix(hex, 16).unwrap()
    }

    fn powercfg_values(plan: Guid) -> PlanValues {
        let read = |s| powercfg_index(plan, s);
        PlanValues {
            active: plan,
            lid_ac: read(PlanSetting::LidAc),
            lid_dc: read(PlanSetting::LidDc),
            sleep_ac: read(PlanSetting::SleepAc),
            sleep_dc: read(PlanSetting::SleepDc),
            saver_threshold_dc: read(PlanSetting::SaverThresholdDc),
        }
    }

    /// A fresh copy, checked to be this process's and not the user's plan before any write goes near it.
    fn fresh_copy(user_plan: Guid) -> TestPlan {
        let swept = TestPlan::sweep().unwrap();
        println!("sweep deleted {swept:?}");
        let copy = TestPlan::new().unwrap();
        println!("copy {}", copy.guid());
        assert_ne!(copy.guid(), user_plan);
        assert_eq!(
            friendly_name(copy.guid()).unwrap(),
            TestPlan::name_for(std::process::id())
        );
        copy
    }

    #[test]
    #[ignore = "real laptop: writes a plan copy that is never active"]
    fn real_write_to_inactive_copy_changes_only_the_copy() {
        let log = Arc::new(MemoryLog::default());
        let mut power = Win32Power::new(log.clone());
        let before = power.values().unwrap();
        println!("user's plan before: {before:?}");
        assert_eq!(
            powercfg_values(before.active),
            before,
            "our read and powercfg agree"
        );

        let copy = fresh_copy(before.active);
        let plan = copy.guid();
        assert_eq!(
            power.values_of(plan).unwrap(),
            PlanValues {
                active: plan,
                ..before
            }
        );
        for key in KEYS {
            let old = power.read(plan, key).unwrap();
            let new = visible_target(key, old);
            power.write(plan, key, new).unwrap();
            assert_eq!(power.read(plan, key), Ok(new));
            assert_eq!(
                powercfg_index(plan, key.into()),
                new,
                "powercfg sees the copy's write"
            );
            println!(
                "copy {plan}: {} {old} to {new}",
                PlanSetting::from(key).name()
            );
        }

        assert_eq!(
            get_active(),
            Ok(before.active),
            "the copy never became active"
        );
        let after = power.values().unwrap();
        println!("user's plan after: {after:?}");
        assert_eq!(after, before);
        assert_eq!(powercfg_values(after.active), before);
        let written = lines(&log);
        assert_eq!(written.len(), 3, "{written:?}");
        assert!(
            written
                .iter()
                .all(|l| l.starts_with("wrote ") && l.contains(&plan.to_string()))
        );

        drop(copy);
        assert!(!power.plans().unwrap().contains(&plan), "the copy is gone");
    }

    #[test]
    #[ignore = "real laptop: makes a plan copy that is never active"]
    fn real_plan_list_ends_with_259() {
        let active = get_active().unwrap();
        let mut before = plan_list(enumerate_at).unwrap();
        println!("plans: {before:?}");
        assert!(before.contains(&active));
        assert_eq!(
            enumerate_at(before.len() as u32),
            Err(ERROR_NO_MORE_ITEMS.0)
        );

        let copy = fresh_copy(active);
        let mut during = plan_list(enumerate_at).unwrap();
        assert!(during.contains(&copy.guid()));
        during.retain(|&p| p != copy.guid());
        before.sort();
        during.sort();
        assert_eq!(during, before);

        let gone = copy.guid();
        drop(copy);
        let mut after = plan_list(enumerate_at).unwrap();
        after.sort();
        assert_eq!(after, before);
        assert!(!after.contains(&gone));
    }

    #[test]
    #[ignore = "real laptop: reads the access check only"]
    fn real_access_check_passes_for_three_keys() {
        let mut power = Win32Power::new(Arc::new(MemoryLog::default()));
        for key in KEYS {
            assert_eq!(power.can_write(key), Ok(()), "{key:?}");
        }
    }

    #[test]
    #[ignore = "real laptop: writes a plan copy that is never active"]
    fn real_copy_as_active_round_trip() {
        let log = Arc::new(MemoryLog::default());
        let mut check = Win32Power::new(log.clone());
        let before = check.values().unwrap();
        let copy = fresh_copy(before.active);
        let plan = copy.guid();

        let mut io = CopyAsActive::over(&copy, Win32Power::new(log.clone())).unwrap();
        assert_eq!(io.active_plan(), Ok(plan));
        assert_eq!(
            io.values().unwrap(),
            PlanValues {
                active: plan,
                ..before
            }
        );
        let mut targets = Vec::new();
        for key in KEYS {
            let old = io.read(plan, key).unwrap();
            let new = visible_target(key, old);
            io.write(plan, key, new).unwrap();
            assert_eq!(
                io.in_force(key),
                old,
                "stored, not in force before set_active"
            );
            targets.push((key, new));
        }
        io.set_active(plan).unwrap();
        assert_eq!(io.set_active_count(), 1);
        let values = io.values().unwrap();
        for (key, new) in targets {
            assert_eq!(io.in_force(key), new);
            assert_eq!(key_value(&values, key), new);
        }

        assert_eq!(
            get_active(),
            Ok(before.active),
            "Windows never switched plans"
        );
        assert_eq!(check.values().unwrap(), before);
        drop(io);
        drop(copy);
        assert!(!check.plans().unwrap().contains(&plan));
    }
}
