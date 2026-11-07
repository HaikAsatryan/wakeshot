//! Lid, AC, battery, active plan, suspend and resume notifications.
//!
//! The callbacks may run on any thread, so they only decode and post (architecture section 4, "Tray threads").
//! Microsoft's pages for `PowerSettingUnregisterNotification` and `PowerUnregisterSuspendResumeNotification` (read
//! 2026-11-03) say nothing about a callback that runs during or after the unregister. So a callback never follows a
//! pointer to its receiver: it looks the receiver up by id in a table. Once a registration's entry is gone, a late
//! callback finds nothing and posts nothing, and a drop waits for a callback that found the entry just before. Here
//! an unregister waited for a running callback (research, "Power notifications: unregister, threads and payloads");
//! nothing relies on that.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use wakeshot_core::event::Event;
use wakeshot_core::settings::Guid;
use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE, WIN32_ERROR};
use windows::Win32::System::Power::{
    DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, GetSystemPowerStatus, HPOWERNOTIFY, POWERBROADCAST_SETTING,
    PowerRegisterSuspendResumeNotification, PowerSettingRegisterNotification,
    PowerSettingUnregisterNotification, PowerUnregisterSuspendResumeNotification,
    SYSTEM_POWER_STATUS,
};
use windows::Win32::System::SystemServices::{
    GUID_ACDC_POWER_SOURCE, GUID_ACTIVE_POWERSCHEME, GUID_BATTERY_PERCENTAGE_REMAINING,
    GUID_LIDSWITCH_STATE_CHANGE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DEVICE_NOTIFY_CALLBACK, PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND, PBT_POWERSETTINGCHANGE,
};
use windows::core::GUID;

use crate::WinError;
use crate::sink::EventSink;

/// The power notifications of one tray run, each posted through the sink:
///
/// - lid (`GUID_LIDSWITCH_STATE_CHANGE`): `Event::Lid`;
/// - power source (`GUID_ACDC_POWER_SOURCE`) and battery percent (`GUID_BATTERY_PERCENTAGE_REMAINING`):
///   `Event::Power`, each with the last value of the other, posted once the power source is known;
/// - active plan (`GUID_ACTIVE_POWERSCHEME`): `Event::ActivePlan`;
/// - suspend and resume: `Event::Suspending` at `PBT_APMSUSPEND`, `Event::Resumed` at `PBT_APMRESUMEAUTOMATIC`.
///
/// Each setting posts its current value right after `start` (Microsoft, `PowerSettingRegisterNotification`
/// Remarks); the lid only once Windows knows a lid device and its state (Power Setting GUIDs page). Drop unregisters
/// all of them; once it returns, nothing more is posted.
#[must_use = "dropping it unregisters every notification"]
pub struct Subscriptions {
    hooks: Hooks,
    failures: Vec<WinError>,
    lost: Arc<AtomicU64>,
}

impl Subscriptions {
    /// Registers every notification. One that fails is left out and listed in `failures`, so a failed plan
    /// notification never costs the lid.
    pub fn start(sink: Arc<dyn EventSink>) -> Subscriptions {
        Self::start_with(Arc::new(Win32Registrar), sink)
    }

    fn start_with(registrar: Arc<dyn Registrar>, sink: Arc<dyn EventSink>) -> Subscriptions {
        let lost = Arc::new(AtomicU64::new(0));
        let feed = Feed {
            sink,
            power: Mutex::new(PowerSeen::default()),
            lost: Arc::clone(&lost),
        };
        let mut hooks = Hooks::new(registrar, Arc::new(feed));
        let failures = WATCHES
            .into_iter()
            .filter_map(|watch| hooks.add(watch).err())
            .collect();
        Subscriptions {
            hooks,
            failures,
            lost,
        }
    }

    /// The registrations that failed at `start`, for the log.
    pub fn failures(&self) -> &[WinError] {
        &self.failures
    }

    /// How many posts have failed so far. A callback cannot log (it must only post), so the loop reads this on its
    /// tick and logs a change.
    pub fn lost_posts(&self) -> u64 {
        self.lost.load(Ordering::Relaxed)
    }
}

impl std::fmt::Debug for Subscriptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriptions")
            .field("id", &self.hooks.id)
            .field("failures", &self.failures)
            .field("lost_posts", &self.lost_posts())
            .finish()
    }
}

/// The lid right now: `Some(true)` closed, `Some(false)` open, `None` when no value came within `timeout_ms` or
/// the registration failed. The hibernate thread's last check: a fresh registration, because its first callback
/// carries the current state. Windows sends none until it knows a lid device and its state, so a PC without a lid
/// gives `None` (Power Setting GUIDs page).
pub fn lid_once(timeout_ms: u64) -> Option<bool> {
    lid_once_with(Arc::new(Win32Registrar), timeout_ms)
}

fn lid_once_with(registrar: Arc<dyn Registrar>, timeout_ms: u64) -> Option<bool> {
    let wait = Arc::new(LidWait::default());
    let mut hooks = Hooks::new(registrar, Arc::clone(&wait) as Arc<dyn Receiver>);
    if hooks
        .add(Watch::Setting(GUID_LIDSWITCH_STATE_CHANGE))
        .is_err()
    {
        return None;
    }
    let lid = wait.wait(Duration::from_millis(timeout_ms));
    drop(hooks);
    lid
}

/// On AC, and the battery percent when there is a battery and Windows knows its charge (`GetSystemPowerStatus`).
pub fn power_status() -> Result<(bool, Option<u8>), WinError> {
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: `status` is a live local the call fills.
    unsafe { GetSystemPowerStatus(&mut status) }.map_err(|e| WinError {
        call: "GetSystemPowerStatus",
        code: WIN32_ERROR::from_error(&e).map_or(e.code().0 as u32, |w| w.0),
    })?;
    Ok(status_values(&status))
}

/// `ACLineStatus` 255 (unknown) counts as AC: the hibernate thread's battery floor check is this read's user, and an
/// unknown source must never let it hibernate. `BatteryFlag` 128 is "no system battery" and 255 "unknown"; both have
/// bit 128 set.
fn status_values(status: &SYSTEM_POWER_STATUS) -> (bool, Option<u8>) {
    const NO_BATTERY: u8 = 128;
    let on_ac = status.ACLineStatus != 0;
    let pct = status.BatteryLifePercent;
    let known = status.BatteryFlag & NO_BATTERY == 0 && pct <= 100;
    (on_ac, known.then_some(pct))
}

/// The registrations `Subscriptions` makes, in order: the power source before the battery percent, so the first
/// `Event::Power` usually carries both.
const WATCHES: [Watch; 5] = [
    Watch::Setting(GUID_LIDSWITCH_STATE_CHANGE),
    Watch::Setting(GUID_ACDC_POWER_SOURCE),
    Watch::Setting(GUID_BATTERY_PERCENTAGE_REMAINING),
    Watch::Setting(GUID_ACTIVE_POWERSCHEME),
    Watch::SuspendResume,
];

/// What one registration watches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Watch {
    Setting(GUID),
    SuspendResume,
}

impl Watch {
    /// The call and its setting, for the log: `WinError::call` is a static string.
    fn register_call(self) -> &'static str {
        match self {
            Watch::Setting(GUID_LIDSWITCH_STATE_CHANGE) => {
                "PowerSettingRegisterNotification(GUID_LIDSWITCH_STATE_CHANGE)"
            }
            Watch::Setting(GUID_ACDC_POWER_SOURCE) => {
                "PowerSettingRegisterNotification(GUID_ACDC_POWER_SOURCE)"
            }
            Watch::Setting(GUID_BATTERY_PERCENTAGE_REMAINING) => {
                "PowerSettingRegisterNotification(GUID_BATTERY_PERCENTAGE_REMAINING)"
            }
            Watch::Setting(GUID_ACTIVE_POWERSCHEME) => {
                "PowerSettingRegisterNotification(GUID_ACTIVE_POWERSCHEME)"
            }
            Watch::Setting(_) => "PowerSettingRegisterNotification",
            Watch::SuspendResume => "PowerRegisterSuspendResumeNotification",
        }
    }
}

/// The Windows side of a registration, so tests can use one that calls back on cue or never.
trait Registrar: Send + Sync {
    /// Registers `watch`; its callbacks must reach `deliver(id, ..)`. Returns the registration handle.
    fn register(&self, watch: Watch, id: usize) -> Result<isize, WinError>;
    fn unregister(&self, watch: Watch, handle: isize);
}

struct Win32Registrar;

impl Registrar for Win32Registrar {
    fn register(&self, watch: Watch, id: usize) -> Result<isize, WinError> {
        let callback: unsafe extern "system" fn(*const c_void, u32, *const c_void) -> u32 =
            match watch {
                Watch::Setting(_) => on_setting,
                Watch::SuspendResume => on_suspend_resume,
            };
        // Leaked on purpose: Microsoft does not say whether Windows copies these or keeps reading them, and 16 bytes
        // per registration costs less than handing Windows a freed pointer.
        let params = Box::leak(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: Some(callback),
            Context: std::ptr::without_provenance_mut(id),
        }));
        let recipient = HANDLE(std::ptr::from_mut(params).cast());
        let mut handle: *mut c_void = std::ptr::null_mut();
        // SAFETY: the parameters are never freed, and the GUID and `handle` live across the call.
        let rc = unsafe {
            match watch {
                Watch::Setting(guid) => PowerSettingRegisterNotification(
                    &guid,
                    DEVICE_NOTIFY_CALLBACK,
                    recipient,
                    &mut handle,
                ),
                Watch::SuspendResume => PowerRegisterSuspendResumeNotification(
                    DEVICE_NOTIFY_CALLBACK,
                    recipient,
                    &mut handle,
                ),
            }
        };
        if rc != ERROR_SUCCESS {
            return Err(WinError {
                call: watch.register_call(),
                code: rc.0,
            });
        }
        Ok(handle.addr() as isize)
    }

    fn unregister(&self, watch: Watch, handle: isize) {
        // A failure changes nothing for us: the table entry goes either way, so a late callback posts nothing.
        // SAFETY: `handle` came from the matching register call and is unregistered once.
        let _ = unsafe {
            match watch {
                Watch::Setting(_) => PowerSettingUnregisterNotification(HPOWERNOTIFY(handle)),
                Watch::SuspendResume => {
                    PowerUnregisterSuspendResumeNotification(HPOWERNOTIFY(handle))
                }
            }
        };
    }
}

unsafe extern "system" fn on_setting(
    context: *const c_void,
    kind: u32,
    setting: *const c_void,
) -> u32 {
    if kind == PBT_POWERSETTINGCHANGE && !setting.is_null() {
        // SAFETY: for PBT_POWERSETTINGCHANGE Windows passes a POWERBROADCAST_SETTING, alive for this call, whose
        // `DataLength` is the size of `Data` (Microsoft's page for the struct).
        let (guid, data) = unsafe { read_setting(setting) };
        deliver(context.addr(), Raw::Setting { guid, data });
    }
    ERROR_SUCCESS.0
}

unsafe extern "system" fn on_suspend_resume(
    context: *const c_void,
    kind: u32,
    _setting: *const c_void,
) -> u32 {
    deliver(context.addr(), Raw::SuspendResume(kind));
    ERROR_SUCCESS.0
}

/// # Safety
///
/// `setting` points to a `POWERBROADCAST_SETTING` followed by `DataLength` bytes, alive for `'a`.
unsafe fn read_setting<'a>(setting: *const c_void) -> (GUID, &'a [u8]) {
    let s = setting.cast::<POWERBROADCAST_SETTING>();
    // SAFETY: the caller's promise. Unaligned reads, since no page promises the block's alignment.
    unsafe {
        let guid = (&raw const (*s).PowerSetting).read_unaligned();
        let len = (&raw const (*s).DataLength).read_unaligned() as usize;
        let data = std::slice::from_raw_parts((&raw const (*s).Data).cast::<u8>(), len);
        (guid, data)
    }
}

/// A callback's payload, before decoding.
#[derive(Clone, Copy, Debug)]
enum Raw<'a> {
    Setting { guid: GUID, data: &'a [u8] },
    SuspendResume(u32),
}

/// Where a registration's callbacks go. It runs on the callback's thread, so it must not block or panic: a panic
/// there aborts the tray.
trait Receiver: Send + Sync {
    fn take(&self, raw: Raw<'_>);
}

/// A receiver as the table holds it. Only the table and the owning `Hooks` hold this `Arc`, so its count tells
/// whether a callback still runs.
struct Slot {
    receiver: Arc<dyn Receiver>,
}

/// The receivers callbacks can reach, by registration id. Ids start at 1, so a null context reaches nothing.
static TABLE: Mutex<BTreeMap<usize, Arc<Slot>>> = Mutex::new(BTreeMap::new());
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn table() -> MutexGuard<'static, BTreeMap<usize, Arc<Slot>>> {
    TABLE.lock().unwrap_or_else(PoisonError::into_inner)
}

fn deliver(id: usize, raw: Raw<'_>) {
    // Cloned under the lock and run after it, so a slow receiver never holds up another registration or a drop.
    let slot = table().get(&id).cloned();
    if let Some(slot) = slot {
        slot.receiver.take(raw);
    }
}

/// One receiver and its registrations. Drop unregisters them, takes the receiver out of the table and waits for a
/// callback that still runs, so nothing reaches the receiver after the drop.
struct Hooks {
    id: usize,
    slot: Arc<Slot>,
    registrar: Arc<dyn Registrar>,
    handles: Vec<(Watch, isize)>,
}

impl Hooks {
    fn new(registrar: Arc<dyn Registrar>, receiver: Arc<dyn Receiver>) -> Hooks {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let slot = Arc::new(Slot { receiver });
        // In the table before the first register call: the first callback may come before that call returns.
        table().insert(id, Arc::clone(&slot));
        Hooks {
            id,
            slot,
            registrar,
            handles: Vec::new(),
        }
    }

    fn add(&mut self, watch: Watch) -> Result<(), WinError> {
        let handle = self.registrar.register(watch, self.id)?;
        self.handles.push((watch, handle));
        Ok(())
    }
}

impl Drop for Hooks {
    fn drop(&mut self) {
        // The table lock is not held across the unregisters: an unregister can wait for a running callback (seen
        // here), and that callback needs the lock.
        for (watch, handle) in self.handles.drain(..) {
            self.registrar.unregister(watch, handle);
        }
        table().remove(&self.id);
        // A callback that found the slot before the removal still holds it. Receivers never block, so this is short.
        while Arc::strong_count(&self.slot) > 1 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

/// Turns one tray run's callbacks into events.
struct Feed {
    sink: Arc<dyn EventSink>,
    power: Mutex<PowerSeen>,
    lost: Arc<AtomicU64>,
}

impl Feed {
    fn post(&self, event: Event) {
        if self.sink.post(event).is_err() {
            self.lost.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn power(&self, update: impl FnOnce(&mut PowerSeen)) {
        let mut seen = self.power.lock().unwrap_or_else(PoisonError::into_inner);
        update(&mut seen);
        // Posted under the lock, so two callbacks on two threads cannot post their readings out of order.
        if let Some(event) = seen.event() {
            self.post(event);
        }
    }
}

impl Receiver for Feed {
    fn take(&self, raw: Raw<'_>) {
        match raw {
            Raw::SuspendResume(kind) => {
                if let Some(event) = decode_suspend_resume(kind) {
                    self.post(event);
                }
            }
            Raw::Setting { guid, data } => match decode_setting(guid, data) {
                Some(Reading::Lid { closed }) => self.post(Event::Lid { closed }),
                Some(Reading::Ac(ac)) => self.power(|seen| seen.ac = Some(ac)),
                Some(Reading::BatteryPct(pct)) => self.power(|seen| seen.battery_pct = Some(pct)),
                Some(Reading::ActivePlan(plan)) => self.post(Event::ActivePlan(plan)),
                None => {}
            },
        }
    }
}

/// The last power source and battery percent seen: Windows reports each alone, `Event::Power` carries both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PowerSeen {
    ac: Option<bool>,
    battery_pct: Option<u8>,
}

impl PowerSeen {
    fn event(self) -> Option<Event> {
        self.ac.map(|ac| Event::Power {
            ac,
            battery_pct: self.battery_pct,
        })
    }
}

/// The first lid value of one fresh registration.
#[derive(Default)]
struct LidWait {
    first: Mutex<Option<bool>>,
    came: Condvar,
}

impl LidWait {
    fn wait(&self, timeout: Duration) -> Option<bool> {
        let first = self.first.lock().unwrap_or_else(PoisonError::into_inner);
        let (first, _) = self
            .came
            .wait_timeout_while(first, timeout, |lid| lid.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        *first
    }
}

impl Receiver for LidWait {
    fn take(&self, raw: Raw<'_>) {
        if let Raw::Setting { guid, data } = raw
            && let Some(Reading::Lid { closed }) = decode_setting(guid, data)
        {
            let mut first = self.first.lock().unwrap_or_else(PoisonError::into_inner);
            if first.is_none() {
                *first = Some(closed);
                self.came.notify_all();
            }
        }
    }
}

/// One decoded setting notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reading {
    Lid { closed: bool },
    Ac(bool),
    BatteryPct(u8),
    ActivePlan(Guid),
}

fn decode_setting(guid: GUID, data: &[u8]) -> Option<Reading> {
    match guid {
        GUID_LIDSWITCH_STATE_CHANGE => decode_lid(data).map(|closed| Reading::Lid { closed }),
        GUID_ACDC_POWER_SOURCE => decode_acdc(data).map(Reading::Ac),
        GUID_BATTERY_PERCENTAGE_REMAINING => decode_battery_pct(data).map(Reading::BatteryPct),
        GUID_ACTIVE_POWERSCHEME => decode_plan_guid(data).map(Reading::ActivePlan),
        _ => None,
    }
}

/// `Some(true)` when closed: 0 closed, 1 open (Power Setting GUIDs page).
fn decode_lid(data: &[u8]) -> Option<bool> {
    match dword(data)? {
        0 => Some(true),
        1 => Some(false),
        _ => None,
    }
}

/// `Some(true)` on AC. 1 is the battery and 2 a short-term source such as a UPS (`SYSTEM_POWER_CONDITION`); 2 counts
/// as battery, since it does not last either.
fn decode_acdc(data: &[u8]) -> Option<bool> {
    match dword(data)? {
        0 => Some(true),
        1 | 2 => Some(false),
        _ => None,
    }
}

/// 0 to 100 (Power Setting GUIDs page); anything else is no reading.
fn decode_battery_pct(data: &[u8]) -> Option<u8> {
    u8::try_from(dword(data)?).ok().filter(|pct| *pct <= 100)
}

/// The payload is the new plan's GUID as Windows lays one out in memory: three little-endian fields, then 8 bytes.
fn decode_plan_guid(data: &[u8]) -> Option<Guid> {
    let b: &[u8; 16] = data.try_into().ok()?;
    let guid = GUID::from_values(
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        u16::from_le_bytes([b[4], b[5]]),
        u16::from_le_bytes([b[6], b[7]]),
        [b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]],
    );
    Some(Guid(guid.to_u128()))
}

/// `PBT_APMRESUMESUSPEND` is left out: it follows `PBT_APMRESUMEAUTOMATIC` when the user is present, so it would post
/// a second `Resumed` for one resume (Microsoft, `PBT_APMRESUMEAUTOMATIC` Remarks).
fn decode_suspend_resume(kind: u32) -> Option<Event> {
    match kind {
        PBT_APMSUSPEND => Some(Event::Suspending),
        PBT_APMRESUMEAUTOMATIC => Some(Event::Resumed),
        _ => None,
    }
}

fn dword(data: &[u8]) -> Option<u32> {
    data.try_into().ok().map(u32::from_le_bytes)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU32;
    use std::thread;
    use std::time::Instant;

    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::System::Power::PowerGetActiveScheme;
    use windows::Win32::UI::WindowsAndMessaging::PBT_APMRESUMESUSPEND;

    use super::*;
    use crate::testing::FakeEventSink;

    /// Balanced, `381b4222-f694-41f0-9685-ff5bb260df2e`.
    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);

    fn le(v: u32) -> [u8; 4] {
        v.to_le_bytes()
    }

    #[test]
    fn decode_lid() {
        assert_eq!(super::decode_lid(&le(0)), Some(true));
        assert_eq!(super::decode_lid(&le(1)), Some(false));
        assert_eq!(super::decode_lid(&le(2)), None);
        assert_eq!(super::decode_lid(&[0, 0]), None);
        assert_eq!(super::decode_lid(&[1, 0, 0, 0, 0, 0, 0, 0]), None);
    }

    #[test]
    fn decode_acdc() {
        assert_eq!(super::decode_acdc(&le(0)), Some(true));
        assert_eq!(super::decode_acdc(&le(1)), Some(false));
        assert_eq!(super::decode_acdc(&le(2)), Some(false));
        assert_eq!(super::decode_acdc(&le(3)), None);
        assert_eq!(super::decode_acdc(&[]), None);
    }

    #[test]
    fn decode_battery_pct() {
        for pct in [0, 1, 57, 100] {
            assert_eq!(super::decode_battery_pct(&le(pct)), Some(pct as u8));
        }
        for bad in [101, 255, 256, u32::MAX] {
            assert_eq!(super::decode_battery_pct(&le(bad)), None, "{bad}");
        }
        assert_eq!(super::decode_battery_pct(&[57]), None);
    }

    /// Balanced's bytes as Windows stores a GUID: `{381B4222-F694-41F0-9685-FF5BB260DF2E}`.
    const BALANCED_BYTES: [u8; 16] = [
        0x22, 0x42, 0x1b, 0x38, 0x94, 0xf6, 0xf0, 0x41, 0x96, 0x85, 0xff, 0x5b, 0xb2, 0x60, 0xdf,
        0x2e,
    ];

    #[test]
    fn decode_plan_guid() {
        assert_eq!(super::decode_plan_guid(&BALANCED_BYTES), Some(BALANCED));
        assert_eq!(
            super::decode_plan_guid(&BALANCED_BYTES).map(|g| g.to_string()),
            Some("381b4222-f694-41f0-9685-ff5bb260df2e".to_owned())
        );
        assert_eq!(super::decode_plan_guid(&BALANCED_BYTES[..15]), None);
        assert_eq!(super::decode_plan_guid(&le(0)), None);
    }

    #[test]
    fn decode_suspend_resume() {
        assert_eq!(
            super::decode_suspend_resume(PBT_APMSUSPEND),
            Some(Event::Suspending)
        );
        assert_eq!(
            super::decode_suspend_resume(PBT_APMRESUMEAUTOMATIC),
            Some(Event::Resumed)
        );
        // PBT_APMPOWERSTATUSCHANGE is 10.
        for other in [PBT_APMRESUMESUSPEND, 10, PBT_POWERSETTINGCHANGE, 0] {
            assert_eq!(super::decode_suspend_resume(other), None, "{other}");
        }
    }

    #[test]
    fn decode_setting_routes_by_guid() {
        assert_eq!(
            decode_setting(GUID_LIDSWITCH_STATE_CHANGE, &le(0)),
            Some(Reading::Lid { closed: true })
        );
        assert_eq!(
            decode_setting(GUID_ACDC_POWER_SOURCE, &le(1)),
            Some(Reading::Ac(false))
        );
        assert_eq!(
            decode_setting(GUID_BATTERY_PERCENTAGE_REMAINING, &le(42)),
            Some(Reading::BatteryPct(42))
        );
        assert_eq!(
            decode_setting(GUID_ACTIVE_POWERSCHEME, &BALANCED_BYTES),
            Some(Reading::ActivePlan(BALANCED))
        );
        // The same payload under another setting's GUID means nothing here.
        let display_state = GUID::from_u128(0x6fe69556_704a_47a0_8f24_c28d936fda47);
        assert_eq!(decode_setting(display_state, &le(0)), None);
    }

    /// The `POWERBROADCAST_SETTING` layout: GUID, `DataLength`, then the data.
    #[repr(C)]
    struct Block {
        guid: GUID,
        len: u32,
        data: [u8; 16],
    }

    fn fire_setting(id: usize, guid: GUID, data: &[u8]) {
        let mut block = Block {
            guid,
            len: data.len() as u32,
            data: [0; 16],
        };
        block.data[..data.len()].copy_from_slice(data);
        // SAFETY: the block has the POWERBROADCAST_SETTING layout with `len` bytes of data and outlives the call.
        unsafe {
            on_setting(
                std::ptr::without_provenance(id),
                PBT_POWERSETTINGCHANGE,
                (&raw const block).cast(),
            )
        };
    }

    fn fire_suspend_resume(id: usize, kind: u32) {
        // SAFETY: the callback reads no pointer for suspend and resume.
        unsafe { on_suspend_resume(std::ptr::without_provenance(id), kind, std::ptr::null()) };
    }

    #[test]
    fn read_setting_reads_the_block() {
        let block = Block {
            guid: GUID_ACTIVE_POWERSCHEME,
            len: 16,
            data: BALANCED_BYTES,
        };
        // SAFETY: as in `fire_setting`.
        let (guid, data) = unsafe { read_setting((&raw const block).cast()) };
        assert_eq!((guid, data), (GUID_ACTIVE_POWERSCHEME, &BALANCED_BYTES[..]));
    }

    /// Stands in for Windows: records each call; `lid_at_register` sends lid values from inside `register`, before it
    /// returns, as Windows may; `fail` makes one watch's register fail.
    #[derive(Default)]
    struct FakeRegistrar {
        registered: Mutex<Vec<(Watch, usize)>>,
        unregistered: Mutex<Vec<(Watch, isize)>>,
        fail: Option<Watch>,
        lid_at_register: Vec<u32>,
    }

    const DENIED: u32 = 5;

    impl FakeRegistrar {
        fn id(&self) -> usize {
            self.registered.lock().unwrap()[0].1
        }

        fn unregistered(&self) -> Vec<(Watch, isize)> {
            self.unregistered.lock().unwrap().clone()
        }
    }

    impl Registrar for FakeRegistrar {
        fn register(&self, watch: Watch, id: usize) -> Result<isize, WinError> {
            if self.fail == Some(watch) {
                return Err(WinError {
                    call: watch.register_call(),
                    code: DENIED,
                });
            }
            let handle = {
                let mut registered = self.registered.lock().unwrap();
                registered.push((watch, id));
                registered.len() as isize
            };
            for lid in &self.lid_at_register {
                fire_setting(id, GUID_LIDSWITCH_STATE_CHANGE, &le(*lid));
            }
            Ok(handle)
        }

        fn unregister(&self, watch: Watch, handle: isize) {
            self.unregistered.lock().unwrap().push((watch, handle));
        }
    }

    fn start_fake(fake: &Arc<FakeRegistrar>) -> (Subscriptions, Arc<FakeEventSink>) {
        let sink = Arc::new(FakeEventSink::default());
        let subs = Subscriptions::start_with(
            Arc::clone(fake) as Arc<dyn Registrar>,
            Arc::clone(&sink) as _,
        );
        (subs, sink)
    }

    #[test]
    fn subscriptions_register_every_watch_in_order() {
        let fake = Arc::new(FakeRegistrar::default());
        let (subs, _sink) = start_fake(&fake);
        let id = fake.id();
        assert_eq!(*fake.registered.lock().unwrap(), WATCHES.map(|w| (w, id)));
        assert_eq!(subs.failures(), []);
    }

    #[test]
    fn subscriptions_post_each_event() {
        let fake = Arc::new(FakeRegistrar::default());
        let (_subs, sink) = start_fake(&fake);
        let id = fake.id();

        fire_setting(id, GUID_LIDSWITCH_STATE_CHANGE, &le(0));
        fire_setting(id, GUID_LIDSWITCH_STATE_CHANGE, &le(1));
        fire_setting(id, GUID_ACTIVE_POWERSCHEME, &BALANCED_BYTES);
        fire_suspend_resume(id, PBT_APMSUSPEND);
        fire_suspend_resume(id, PBT_APMRESUMEAUTOMATIC);
        fire_suspend_resume(id, PBT_APMRESUMESUSPEND);
        // A value outside the documented ones posts nothing.
        fire_setting(id, GUID_LIDSWITCH_STATE_CHANGE, &le(7));

        assert_eq!(
            sink.events(),
            [
                Event::Lid { closed: true },
                Event::Lid { closed: false },
                Event::ActivePlan(BALANCED),
                Event::Suspending,
                Event::Resumed,
            ]
        );
    }

    #[test]
    fn power_waits_for_the_source_and_keeps_the_last_of_each() {
        let fake = Arc::new(FakeRegistrar::default());
        let (_subs, sink) = start_fake(&fake);
        let id = fake.id();

        fire_setting(id, GUID_BATTERY_PERCENTAGE_REMAINING, &le(80));
        assert_eq!(sink.events(), [], "no power source yet");
        fire_setting(id, GUID_ACDC_POWER_SOURCE, &le(1));
        fire_setting(id, GUID_BATTERY_PERCENTAGE_REMAINING, &le(79));
        fire_setting(id, GUID_ACDC_POWER_SOURCE, &le(0));
        fire_setting(id, GUID_ACDC_POWER_SOURCE, &le(2));

        let power = |ac, pct| Event::Power {
            ac,
            battery_pct: Some(pct),
        };
        assert_eq!(
            sink.events(),
            [
                power(false, 80),
                power(false, 79),
                power(true, 79),
                power(false, 79)
            ]
        );
    }

    #[test]
    fn power_without_a_battery_reading_has_no_percent() {
        let fake = Arc::new(FakeRegistrar::default());
        let (_subs, sink) = start_fake(&fake);
        fire_setting(fake.id(), GUID_ACDC_POWER_SOURCE, &le(0));
        assert_eq!(
            sink.events(),
            [Event::Power {
                ac: true,
                battery_pct: None
            }]
        );
    }

    #[test]
    fn failed_registration_is_listed_and_the_rest_still_post() {
        let plan = Watch::Setting(GUID_ACTIVE_POWERSCHEME);
        let fake = Arc::new(FakeRegistrar {
            fail: Some(plan),
            ..FakeRegistrar::default()
        });
        let (subs, sink) = start_fake(&fake);
        assert_eq!(
            subs.failures(),
            [WinError {
                call: "PowerSettingRegisterNotification(GUID_ACTIVE_POWERSCHEME)",
                code: DENIED,
            }]
        );
        assert_eq!(fake.registered.lock().unwrap().len(), 4);
        fire_setting(fake.id(), GUID_LIDSWITCH_STATE_CHANGE, &le(0));
        assert_eq!(sink.events(), [Event::Lid { closed: true }]);
    }

    #[test]
    fn failed_post_is_counted() {
        let fake = Arc::new(FakeRegistrar::default());
        let sink = Arc::new(FakeEventSink {
            fail_with: Some(WinError {
                call: "PostMessageW",
                code: 1816,
            }),
            ..FakeEventSink::default()
        });
        let subs = Subscriptions::start_with(Arc::clone(&fake) as _, sink);
        assert_eq!(subs.lost_posts(), 0);
        fire_setting(fake.id(), GUID_LIDSWITCH_STATE_CHANGE, &le(0));
        fire_suspend_resume(fake.id(), PBT_APMSUSPEND);
        assert_eq!(subs.lost_posts(), 2);
    }

    #[test]
    fn drop_unregisters_all_and_later_callbacks_post_nothing() {
        let fake = Arc::new(FakeRegistrar::default());
        let (subs, sink) = start_fake(&fake);
        let id = fake.id();
        drop(subs);

        let unregistered = fake.unregistered();
        assert_eq!(
            unregistered,
            WATCHES.into_iter().zip(1..).collect::<Vec<_>>()
        );
        assert!(!table().contains_key(&id));

        fire_setting(id, GUID_LIDSWITCH_STATE_CHANGE, &le(0));
        fire_suspend_resume(id, PBT_APMSUSPEND);
        assert_eq!(sink.events(), []);
    }

    /// Counts calls and sleeps inside one, so a drop can start while it runs.
    #[derive(Default)]
    struct Slow {
        started: AtomicU32,
        finished: AtomicU32,
    }

    impl Receiver for Slow {
        fn take(&self, _raw: Raw<'_>) {
            self.started.fetch_add(1, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(100));
            self.finished.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn drop_waits_for_a_running_callback() {
        let slow = Arc::new(Slow::default());
        let fake = Arc::new(FakeRegistrar::default());
        let mut hooks = Hooks::new(fake as _, Arc::clone(&slow) as _);
        hooks.add(Watch::SuspendResume).unwrap();
        let id = hooks.id;

        let callback = thread::spawn(move || fire_suspend_resume(id, PBT_APMSUSPEND));
        while slow.started.load(Ordering::SeqCst) == 0 {
            thread::yield_now();
        }
        drop(hooks);
        assert_eq!(
            slow.finished.load(Ordering::SeqCst),
            1,
            "drop returned while a callback ran"
        );
        callback.join().unwrap();

        fire_suspend_resume(id, PBT_APMSUSPEND);
        assert_eq!(slow.started.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn lid_once_times_out() {
        let fake = Arc::new(FakeRegistrar::default());
        let start = Instant::now();
        assert_eq!(lid_once_with(Arc::clone(&fake) as _, 50), None);
        assert!(start.elapsed() >= Duration::from_millis(50));
        assert_eq!(
            fake.unregistered(),
            [(Watch::Setting(GUID_LIDSWITCH_STATE_CHANGE), 1)]
        );
        assert!(!table().contains_key(&fake.id()));
    }

    #[test]
    fn lid_once_takes_the_first_value_even_before_register_returns() {
        let fake = Arc::new(FakeRegistrar {
            lid_at_register: vec![0, 1],
            ..FakeRegistrar::default()
        });
        let start = Instant::now();
        assert_eq!(lid_once_with(fake as _, 2000), Some(true));
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn lid_once_takes_a_value_from_another_thread() {
        /// Calls back 30 ms after `register`, from its own thread.
        struct Late;
        impl Registrar for Late {
            fn register(&self, _watch: Watch, id: usize) -> Result<isize, WinError> {
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(30));
                    fire_setting(id, GUID_LIDSWITCH_STATE_CHANGE, &le(1));
                });
                Ok(1)
            }
            fn unregister(&self, _watch: Watch, _handle: isize) {}
        }
        assert_eq!(lid_once_with(Arc::new(Late), 2000), Some(false));
    }

    #[test]
    fn lid_once_failed_registration_is_unknown() {
        let lid = Watch::Setting(GUID_LIDSWITCH_STATE_CHANGE);
        let fake = Arc::new(FakeRegistrar {
            fail: Some(lid),
            ..FakeRegistrar::default()
        });
        let start = Instant::now();
        assert_eq!(lid_once_with(fake as _, 2000), None);
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    fn status(ac: u8, flag: u8, pct: u8) -> SYSTEM_POWER_STATUS {
        SYSTEM_POWER_STATUS {
            ACLineStatus: ac,
            BatteryFlag: flag,
            BatteryLifePercent: pct,
            ..SYSTEM_POWER_STATUS::default()
        }
    }

    #[test]
    fn status_values_read_the_documented_flags() {
        const CHARGING_HIGH: u8 = 8 | 1;
        assert_eq!(
            status_values(&status(1, CHARGING_HIGH, 97)),
            (true, Some(97))
        );
        assert_eq!(status_values(&status(0, 0, 40)), (false, Some(40)));
        assert_eq!(
            status_values(&status(255, 0, 40)),
            (true, Some(40)),
            "unknown source counts as AC"
        );
        assert_eq!(
            status_values(&status(1, 128, 255)),
            (true, None),
            "no battery"
        );
        assert_eq!(
            status_values(&status(0, 255, 255)),
            (false, None),
            "unknown battery"
        );
        assert_eq!(
            status_values(&status(0, 2, 255)),
            (false, None),
            "unknown percent"
        );
    }

    #[test]
    fn power_status_reads() {
        let (_ac, pct) = power_status().unwrap();
        assert!(pct.is_none_or(|p| p <= 100), "{pct:?}");
    }

    fn active_plan() -> Guid {
        let mut p: *mut GUID = std::ptr::null_mut();
        // SAFETY: `p` is a live out pointer; the reserved root key is null as the docs require.
        let rc = unsafe { PowerGetActiveScheme(None, &mut p) };
        assert_eq!(rc, ERROR_SUCCESS);
        assert!(!p.is_null());
        // SAFETY: Windows allocated the GUID with LocalAlloc; it is read once and freed once.
        let guid = unsafe { *p };
        // SAFETY: as above; nothing reads `p` after this.
        unsafe { LocalFree(Some(HLOCAL(p.cast()))) };
        Guid(guid.to_u128())
    }

    fn wait_for(
        sink: &FakeEventSink,
        timeout: Duration,
        done: impl Fn(&[Event]) -> bool,
    ) -> Vec<Event> {
        let start = Instant::now();
        loop {
            let events = sink.events();
            if done(&events) || start.elapsed() > timeout {
                return events;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    #[ignore = "real: needs a lid that is open; run with `-- --ignored power::system::`"]
    fn real_registration_gives_first_values() {
        let sink = Arc::new(FakeEventSink::default());
        let subs = Subscriptions::start(Arc::clone(&sink) as _);
        assert_eq!(subs.failures(), []);
        wait_for(&sink, Duration::from_secs(2), |events| {
            let has = |f: fn(&Event) -> bool| events.iter().any(f);
            has(|e| matches!(e, Event::Lid { .. }))
                && has(|e| {
                    matches!(
                        e,
                        Event::Power {
                            battery_pct: Some(_),
                            ..
                        }
                    )
                })
                && has(|e| matches!(e, Event::ActivePlan(_)))
        });
        drop(subs);
        let events = sink.events();
        let (ac, battery_pct) = power_status().unwrap();

        assert!(events.contains(&Event::Lid { closed: false }), "{events:?}");
        let last_power = events.iter().rfind(|e| matches!(e, Event::Power { .. }));
        assert_eq!(
            last_power,
            Some(&Event::Power { ac, battery_pct }),
            "{events:?}"
        );
        assert!(
            events.contains(&Event::ActivePlan(active_plan())),
            "{events:?}"
        );
        thread::sleep(Duration::from_millis(100));
        assert_eq!(sink.events(), events, "a post after the drop");
    }

    #[test]
    #[ignore = "real: needs a lid that is open; run with `-- --ignored power::system::`"]
    fn real_lid_once_returns_open_here() {
        let start = Instant::now();
        assert_eq!(lid_once(2000), Some(false));
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "{:?}",
            start.elapsed()
        );
    }
}
