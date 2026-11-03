//! The one error type of the Windows layer (architecture section 8).
//!
//! Owner: contracts card.

use wakeshot_core::command::Failure;

/// A failed Windows call: which call, and its Win32 error or HRESULT as `u32`.
///
/// Owner: power track (Phase 3). Every function of this crate that calls Windows returns `Result<T, WinError>`; the
/// core gets the same two fields as `Failure`, so it can log the call and pick the fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WinError {
    pub call: &'static str,
    pub code: u32,
}

impl From<WinError> for Failure {
    fn from(e: WinError) -> Self {
        Failure {
            call: e.call,
            code: e.code,
        }
    }
}
