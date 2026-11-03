//! Power requests: create, set, clear, retake; real and dry run.
//!
//! Owner: `p3-requests`.
//!
//! One request object per tray (architecture section 5, "Power requests"). `PowerSetRequest` adds one to a count per
//! type and `PowerClearRequest` takes one off (Microsoft docs), so every type here is set at most once: a second set
//! would need a second clear before Windows lets the PC sleep again. `PowerRequests`, `DryRunRequests` and the tests
//! share one rule engine over a small call seam, so the dry-run log and the tests see exactly the calls a real run
//! makes.

#[cfg(any(test, feature = "test-support"))]
use std::collections::VecDeque;
use std::fmt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::sync::Arc;

use windows::Win32::Foundation::{HANDLE, WIN32_ERROR};
use windows::Win32::System::Power::{
    POWER_REQUEST_TYPE, PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired,
    PowerRequestExecutionRequired, PowerRequestSystemRequired, PowerSetRequest,
};
use windows::Win32::System::SystemServices::POWER_REQUEST_CONTEXT_VERSION;
use windows::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};
use windows::core::PWSTR;

use crate::WinError;
use crate::log::{Level, Log};

const AREA: &str = "power";

const REASON: &str = "Wakeshot: keeping the PC awake for running work";

/// The tray's power requests, run by the loop thread for `Command::Requests` and `Command::Retake`. The runner answers
/// both with `Outcome::Requests(result.map_err(Failure::from))` (review m7).
///
/// A call that fails part way clears what it set and returns the error. What it cleared stays cleared, and a type
/// whose clear failed still counts as on, so the next call or the drop clears it again: on a failure the PC holds
/// less, never more.
///
/// Owner: power track (Phase 3).
pub trait RequestApi {
    /// `system` sets or clears SYSTEM and EXECUTION together, `display` sets or clears DISPLAY; only what differs from
    /// what is on is changed. Clears run first, all of them even when one fails; sets run only after every clear
    /// worked.
    fn set(&mut self, system: bool, display: bool) -> Result<(), WinError>;

    /// After a resume: clears each type that is on, then sets the same types again on the same handle, the tested
    /// form (research, "Lid and sleep spike", "Resume", H4). With nothing on it calls nothing.
    fn retake(&mut self) -> Result<(), WinError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    System,
    Execution,
    Display,
}

impl Kind {
    /// The order of every clear and set: H4 cleared and set SYSTEM before EXECUTION.
    const ALL: [Self; 3] = [Self::System, Self::Execution, Self::Display];

    fn index(self) -> usize {
        match self {
            Self::System => 0,
            Self::Execution => 1,
            Self::Display => 2,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::System => "SYSTEM",
            Self::Execution => "EXECUTION",
            Self::Display => "DISPLAY",
        }
    }

    fn request_type(self) -> POWER_REQUEST_TYPE {
        match self {
            Self::System => PowerRequestSystemRequired,
            Self::Execution => PowerRequestExecutionRequired,
            Self::Display => PowerRequestDisplayRequired,
        }
    }
}

/// The four calls the rules make, so the rules are tested without Windows.
trait Calls {
    type Handle;
    fn create(&mut self) -> Result<Self::Handle, WinError>;
    fn set(&mut self, handle: &Self::Handle, kind: Kind) -> Result<(), WinError>;
    fn clear(&mut self, handle: &Self::Handle, kind: Kind) -> Result<(), WinError>;
    fn close(&mut self, handle: Self::Handle);
}

/// The rules of `RequestApi` over any `Calls`. `on` is what this object believes Windows holds for it, per `Kind`.
struct Engine<C: Calls> {
    calls: C,
    handle: Option<C::Handle>,
    on: [bool; 3],
}

impl<C: Calls> Engine<C> {
    fn new(calls: C) -> Self {
        Self {
            calls,
            handle: None,
            on: [false; 3],
        }
    }

    fn is_on(&self, kind: Kind) -> bool {
        self.on[kind.index()]
    }

    fn set(&mut self, system: bool, display: bool) -> Result<(), WinError> {
        let wanted = |kind: Kind| match kind {
            Kind::System | Kind::Execution => system,
            Kind::Display => display,
        };
        let clears: Vec<Kind> = Kind::ALL
            .into_iter()
            .filter(|&k| self.is_on(k) && !wanted(k))
            .collect();
        let sets: Vec<Kind> = Kind::ALL
            .into_iter()
            .filter(|&k| !self.is_on(k) && wanted(k))
            .collect();
        self.run(&clears, &sets)
    }

    fn retake(&mut self) -> Result<(), WinError> {
        let on: Vec<Kind> = Kind::ALL.into_iter().filter(|&k| self.is_on(k)).collect();
        self.run(&on, &on)
    }

    fn run(&mut self, clears: &[Kind], sets: &[Kind]) -> Result<(), WinError> {
        if clears.is_empty() && sets.is_empty() {
            return Ok(());
        }
        let Self { calls, handle, on } = self;
        let handle = match handle {
            Some(handle) => handle,
            None => handle.insert(calls.create()?),
        };

        // Every clear is tried, so one failure does not leave the other types holding the PC awake.
        let mut failed = None;
        for &kind in clears {
            match calls.clear(handle, kind) {
                Ok(()) => on[kind.index()] = false,
                Err(e) => {
                    failed.get_or_insert(e);
                }
            }
        }
        if let Some(e) = failed {
            return Err(e);
        }

        for (done, &kind) in sets.iter().enumerate() {
            if let Err(e) = calls.set(handle, kind) {
                for &undo in sets[..done].iter().rev() {
                    if calls.clear(handle, undo).is_ok() {
                        on[undo.index()] = false;
                    }
                }
                return Err(e);
            }
            on[kind.index()] = true;
        }
        Ok(())
    }
}

impl<C: Calls> Drop for Engine<C> {
    fn drop(&mut self) {
        let Some(handle) = self.handle.take() else {
            return;
        };
        for kind in Kind::ALL {
            if self.is_on(kind) {
                // Nobody is left to act on a failed clear; the close below cleans up the object (Microsoft docs).
                let _ = self.calls.clear(&handle, kind);
                self.on[kind.index()] = false;
            }
        }
        self.calls.close(handle);
    }
}

impl<C: Calls> fmt::Debug for Engine<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Engine")
            .field("created", &self.handle.is_some())
            .field("on", &self.on)
            .finish_non_exhaustive()
    }
}

/// The request object and its reason text. The text lives as long as the handle: the docs do not say whether
/// `PowerCreateRequest` copies it.
struct Win32Request {
    handle: OwnedHandle,
    _reason: Vec<u16>,
}

impl Win32Request {
    fn raw(&self) -> HANDLE {
        HANDLE(self.handle.as_raw_handle())
    }
}

struct Win32Calls;

impl Calls for Win32Calls {
    type Handle = Win32Request;

    fn create(&mut self) -> Result<Win32Request, WinError> {
        let mut reason: Vec<u16> = REASON.encode_utf16().chain(Some(0)).collect();
        let context = REASON_CONTEXT {
            Version: POWER_REQUEST_CONTEXT_VERSION,
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: PWSTR(reason.as_mut_ptr()),
            },
        };
        // SAFETY: `context` and the text it points to outlive the call.
        let raw = unsafe { PowerCreateRequest(&context) }
            .map_err(|e| win_error("PowerCreateRequest", &e))?;
        // SAFETY: a valid handle the call just returned, owned by nothing else; `OwnedHandle` closes it with
        // `CloseHandle`, which the docs name as the way to free the object.
        let handle = unsafe { OwnedHandle::from_raw_handle(raw.0) };
        Ok(Win32Request {
            handle,
            _reason: reason,
        })
    }

    fn set(&mut self, request: &Win32Request, kind: Kind) -> Result<(), WinError> {
        // SAFETY: the handle is open while `request` lives.
        unsafe { PowerSetRequest(request.raw(), kind.request_type()) }
            .map_err(|e| win_error("PowerSetRequest", &e))
    }

    fn clear(&mut self, request: &Win32Request, kind: Kind) -> Result<(), WinError> {
        // SAFETY: the handle is open while `request` lives.
        unsafe { PowerClearRequest(request.raw(), kind.request_type()) }
            .map_err(|e| win_error("PowerClearRequest", &e))
    }

    fn close(&mut self, request: Win32Request) {
        drop(request);
    }
}

/// Win32 errors keep their Win32 code (6, 87), so a log line matches the docs; other failures keep the HRESULT.
fn win_error(call: &'static str, error: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: WIN32_ERROR::from_error(error).map_or(error.code().0 as u32, |e| e.0),
    }
}

/// The real requests: one `PowerCreateRequest` object, made at the first call that sets a type. Drop clears every
/// type that is on and closes the handle.
#[derive(Debug)]
pub struct PowerRequests(Engine<Win32Calls>);

impl PowerRequests {
    pub fn new() -> Self {
        Self(Engine::new(Win32Calls))
    }
}

impl Default for PowerRequests {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestApi for PowerRequests {
    fn set(&mut self, system: bool, display: bool) -> Result<(), WinError> {
        self.0.set(system, display)
    }

    fn retake(&mut self) -> Result<(), WinError> {
        self.0.retake()
    }
}

struct DryRunCalls {
    log: Arc<dyn Log>,
}

impl DryRunCalls {
    fn line(&self, text: &str) {
        self.log
            .line(Level::Info, AREA, &format!("dry run: {text}"));
    }
}

impl Calls for DryRunCalls {
    type Handle = ();

    fn create(&mut self) -> Result<(), WinError> {
        self.line("PowerCreateRequest");
        Ok(())
    }

    fn set(&mut self, (): &(), kind: Kind) -> Result<(), WinError> {
        self.line(&format!("PowerSetRequest {}", kind.name()));
        Ok(())
    }

    fn clear(&mut self, (): &(), kind: Kind) -> Result<(), WinError> {
        self.line(&format!("PowerClearRequest {}", kind.name()));
        Ok(())
    }

    fn close(&mut self, (): ()) {
        self.line("CloseHandle on the power request");
    }
}

/// Logs each call `PowerRequests` would make, under the same rules, and makes none (track note 8). Every call
/// succeeds.
pub struct DryRunRequests(Engine<DryRunCalls>);

impl DryRunRequests {
    pub fn new(log: Arc<dyn Log>) -> Self {
        Self(Engine::new(DryRunCalls { log }))
    }
}

impl fmt::Debug for DryRunRequests {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DryRunRequests").field(&self.0).finish()
    }
}

impl RequestApi for DryRunRequests {
    fn set(&mut self, system: bool, display: bool) -> Result<(), WinError> {
        self.0.set(system, display)
    }

    fn retake(&mut self) -> Result<(), WinError> {
        self.0.retake()
    }
}

/// One call a `FakeRequests` saw.
#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestCall {
    Set { system: bool, display: bool },
    Retake,
}

/// Records each call. Each call returns the next entry of `results`, and `Ok` once it is empty, so a test can fail
/// any one call.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct FakeRequests {
    pub calls: Vec<RequestCall>,
    pub results: VecDeque<Result<(), WinError>>,
}

#[cfg(any(test, feature = "test-support"))]
impl FakeRequests {
    fn answer(&mut self, call: RequestCall) -> Result<(), WinError> {
        self.calls.push(call);
        self.results.pop_front().unwrap_or(Ok(()))
    }
}

#[cfg(any(test, feature = "test-support"))]
impl RequestApi for FakeRequests {
    fn set(&mut self, system: bool, display: bool) -> Result<(), WinError> {
        self.answer(RequestCall::Set { system, display })
    }

    fn retake(&mut self) -> Result<(), WinError> {
        self.answer(RequestCall::Retake)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::log::MemoryLog;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Call {
        Create,
        Set(Kind),
        Clear(Kind),
        Close,
    }

    use Call::{Clear, Close, Create, Set};
    use Kind::{Display, Execution, System};

    #[derive(Debug, Default)]
    struct Record {
        calls: Vec<Call>,
        /// Calls that fail while they are listed here.
        failing: Vec<Call>,
    }

    /// A recording backend. Clones share one record, so the test still reads it after the engine is dropped.
    #[derive(Clone, Debug, Default)]
    struct Recorder(Rc<RefCell<Record>>);

    impl Recorder {
        fn take(&self) -> Vec<Call> {
            std::mem::take(&mut self.0.borrow_mut().calls)
        }

        fn fail(&self, call: Call) {
            self.0.borrow_mut().failing.push(call);
        }

        fn heal(&self) {
            self.0.borrow_mut().failing.clear();
        }

        fn answer(&self, call: Call, name: &'static str) -> Result<(), WinError> {
            let mut record = self.0.borrow_mut();
            record.calls.push(call);
            if record.failing.contains(&call) {
                Err(failure(name))
            } else {
                Ok(())
            }
        }
    }

    impl Calls for Recorder {
        type Handle = ();

        fn create(&mut self) -> Result<(), WinError> {
            self.answer(Create, "PowerCreateRequest")
        }

        fn set(&mut self, (): &(), kind: Kind) -> Result<(), WinError> {
            self.answer(Set(kind), "PowerSetRequest")
        }

        fn clear(&mut self, (): &(), kind: Kind) -> Result<(), WinError> {
            self.answer(Clear(kind), "PowerClearRequest")
        }

        fn close(&mut self, (): ()) {
            self.0.borrow_mut().calls.push(Close);
        }
    }

    fn failure(call: &'static str) -> WinError {
        WinError { call, code: 6 }
    }

    fn engine() -> (Engine<Recorder>, Recorder) {
        let recorder = Recorder::default();
        (Engine::new(recorder.clone()), recorder)
    }

    #[test]
    fn set_changes_only_what_differs() {
        let (mut requests, record) = engine();

        assert_eq!(requests.set(false, false), Ok(()));
        assert_eq!(
            record.take(),
            [],
            "nothing on, nothing wanted: not even a create"
        );

        assert_eq!(requests.set(true, false), Ok(()));
        assert_eq!(record.take(), [Create, Set(System), Set(Execution)]);

        assert_eq!(requests.set(true, false), Ok(()));
        assert_eq!(record.take(), [], "a second set would raise the count");

        assert_eq!(requests.set(true, true), Ok(()));
        assert_eq!(record.take(), [Set(Display)]);

        assert_eq!(requests.set(true, false), Ok(()));
        assert_eq!(record.take(), [Clear(Display)]);

        assert_eq!(requests.set(false, false), Ok(()));
        assert_eq!(record.take(), [Clear(System), Clear(Execution)]);

        assert_eq!(requests.set(true, true), Ok(()));
        assert_eq!(
            record.take(),
            [Set(System), Set(Execution), Set(Display)],
            "one handle for the tray's life: no second create"
        );

        drop(requests);
        assert_eq!(
            record.take(),
            [Clear(System), Clear(Execution), Clear(Display), Close]
        );
    }

    #[test]
    fn clears_run_before_sets() {
        let (mut requests, record) = engine();
        requests.set(false, true).unwrap();
        record.take();

        assert_eq!(requests.set(true, false), Ok(()));
        assert_eq!(record.take(), [Clear(Display), Set(System), Set(Execution)]);
    }

    #[test]
    fn display_only_when_asked() {
        let (mut requests, record) = engine();
        requests.set(true, false).unwrap();
        requests.retake().unwrap();
        requests.set(false, false).unwrap();
        requests.set(true, false).unwrap();
        requests.retake().unwrap();
        let calls = record.take();
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, Set(Display) | Clear(Display))),
            "{calls:?}"
        );

        requests.set(true, true).unwrap();
        assert_eq!(record.take(), [Set(Display)]);
        requests.set(true, false).unwrap();
        assert_eq!(record.take(), [Clear(Display)]);
    }

    #[test]
    fn retake_clears_then_sets_each_on_type() {
        let (mut requests, record) = engine();
        requests.set(true, false).unwrap();
        record.take();

        assert_eq!(requests.retake(), Ok(()));
        assert_eq!(
            record.take(),
            [Clear(System), Clear(Execution), Set(System), Set(Execution)]
        );

        requests.set(true, true).unwrap();
        record.take();
        assert_eq!(requests.retake(), Ok(()));
        assert_eq!(
            record.take(),
            [
                Clear(System),
                Clear(Execution),
                Clear(Display),
                Set(System),
                Set(Execution),
                Set(Display),
            ],
            "the same handle: no create"
        );

        // Still counted as on, so the next change clears them once.
        requests.set(false, false).unwrap();
        assert_eq!(
            record.take(),
            [Clear(System), Clear(Execution), Clear(Display)]
        );
    }

    #[test]
    fn retake_with_nothing_on_does_nothing() {
        let (mut requests, record) = engine();
        assert_eq!(requests.retake(), Ok(()));
        assert_eq!(record.take(), [], "no create either");

        requests.set(true, true).unwrap();
        requests.set(false, false).unwrap();
        record.take();
        assert_eq!(requests.retake(), Ok(()));
        assert_eq!(record.take(), []);
    }

    #[test]
    fn failure_part_way_rolls_back() {
        // A set: what this call set is cleared again.
        let (mut requests, record) = engine();
        record.fail(Set(Execution));
        assert_eq!(requests.set(true, true), Err(failure("PowerSetRequest")));
        assert_eq!(
            record.take(),
            [Create, Set(System), Set(Execution), Clear(System)]
        );
        record.heal();
        requests.set(true, true).unwrap();
        assert_eq!(
            record.take(),
            [Set(System), Set(Execution), Set(Display)],
            "nothing was left counted as on"
        );

        // A retake: its clears stand, and what it set again is cleared.
        record.fail(Set(Display));
        assert_eq!(requests.retake(), Err(failure("PowerSetRequest")));
        assert_eq!(
            record.take(),
            [
                Clear(System),
                Clear(Execution),
                Clear(Display),
                Set(System),
                Set(Execution),
                Set(Display),
                Clear(Execution),
                Clear(System),
            ]
        );
        record.heal();
        assert_eq!(requests.retake(), Ok(()));
        assert_eq!(record.take(), [], "nothing is on after the rollback");
        requests.set(true, false).unwrap();
        assert_eq!(record.take(), [Set(System), Set(Execution)]);
    }

    #[test]
    fn failed_clear_keeps_the_type_on() {
        let (mut requests, record) = engine();
        requests.set(true, true).unwrap();
        record.take();

        record.fail(Clear(System));
        assert_eq!(
            requests.set(false, false),
            Err(failure("PowerClearRequest"))
        );
        assert_eq!(
            record.take(),
            [Clear(System), Clear(Execution), Clear(Display)],
            "every clear is tried"
        );

        // A failed clear also stops the sets of the same call.
        assert_eq!(requests.set(false, true), Err(failure("PowerClearRequest")));
        assert_eq!(record.take(), [Clear(System)], "no Set(Display)");

        record.heal();
        requests.set(false, false).unwrap();
        assert_eq!(record.take(), [Clear(System)], "only SYSTEM was still on");
        drop(requests);
        assert_eq!(record.take(), [Close]);
    }

    #[test]
    fn failed_create_is_tried_again() {
        let (mut requests, record) = engine();
        record.fail(Create);
        assert_eq!(
            requests.set(true, false),
            Err(failure("PowerCreateRequest"))
        );
        assert_eq!(record.take(), [Create]);
        assert_eq!(requests.retake(), Ok(()), "nothing is on");

        record.heal();
        requests.set(true, false).unwrap();
        assert_eq!(record.take(), [Create, Set(System), Set(Execution)]);
    }

    #[test]
    fn drop_clears_all() {
        let (mut requests, record) = engine();
        requests.set(true, true).unwrap();
        record.take();
        drop(requests);
        assert_eq!(
            record.take(),
            [Clear(System), Clear(Execution), Clear(Display), Close]
        );

        // A failed clear still ends in the close.
        let (mut requests, record) = engine();
        requests.set(true, false).unwrap();
        record.take();
        record.fail(Clear(System));
        drop(requests);
        assert_eq!(record.take(), [Clear(System), Clear(Execution), Close]);

        // Never used: no handle, nothing to close.
        let (requests, record) = engine();
        drop(requests);
        assert_eq!(record.take(), []);
    }

    #[test]
    fn dry_run_logs_each_call_and_makes_none() {
        let log = Arc::new(MemoryLog::default());
        let mut requests = DryRunRequests::new(log.clone());
        requests.set(true, false).unwrap();
        requests.set(true, false).unwrap();
        requests.retake().unwrap();
        drop(requests);

        let lines: Vec<(Level, &str, String)> = log.lines.lock().unwrap().clone();
        let text: Vec<&str> = lines.iter().map(|(_, _, t)| t.as_str()).collect();
        assert_eq!(
            text,
            [
                "dry run: PowerCreateRequest",
                "dry run: PowerSetRequest SYSTEM",
                "dry run: PowerSetRequest EXECUTION",
                "dry run: PowerClearRequest SYSTEM",
                "dry run: PowerClearRequest EXECUTION",
                "dry run: PowerSetRequest SYSTEM",
                "dry run: PowerSetRequest EXECUTION",
                "dry run: PowerClearRequest SYSTEM",
                "dry run: PowerClearRequest EXECUTION",
                "dry run: CloseHandle on the power request",
            ]
        );
        assert!(
            lines
                .iter()
                .all(|(l, a, _)| (*l, *a) == (Level::Info, AREA))
        );
    }

    #[test]
    fn fake_records_calls_and_scripted_failures() {
        let error = failure("PowerSetRequest");
        let mut fake = FakeRequests {
            results: VecDeque::from([Ok(()), Err(error)]),
            ..FakeRequests::default()
        };
        let api: &mut dyn RequestApi = &mut fake;
        assert_eq!(api.set(true, false), Ok(()));
        assert_eq!(api.retake(), Err(error));
        assert_eq!(
            api.set(false, false),
            Ok(()),
            "Ok once the script is used up"
        );
        assert_eq!(
            fake.calls,
            [
                RequestCall::Set {
                    system: true,
                    display: false
                },
                RequestCall::Retake,
                RequestCall::Set {
                    system: false,
                    display: false
                },
            ]
        );
    }

    #[test]
    fn win32_errors_keep_their_code() {
        let error = windows::core::Error::from(WIN32_ERROR(6));
        assert_eq!(
            win_error("PowerSetRequest", &error),
            WinError {
                call: "PowerSetRequest",
                code: 6
            }
        );
    }

    /// Real: holds one SYSTEM request for well under a second; nothing persists. Judged only by the calls' results,
    /// never by `powercfg /requests` (architecture section 5).
    #[test]
    fn request_sets_and_clears() {
        let mut requests = PowerRequests::new();
        assert_eq!(requests.set(true, false), Ok(()));
        assert_eq!(requests.retake(), Ok(()));
        assert_eq!(requests.set(false, false), Ok(()));
        drop(requests);
    }
}
