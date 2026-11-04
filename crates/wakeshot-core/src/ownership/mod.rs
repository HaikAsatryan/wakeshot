//! Settings ownership: the one apply, the one restore routine and the ownership check every process runs
//! (architecture section 5), and the reset of a lost journal the tray runs, generic over two seams so the stop-point
//! tests drive the real order of steps.
//!
//! Owner: contracts card for this module list and `io`; core track (Phase 2) for the routines and the fake;
//! `p2-6-record-lost` added `reset`. The power track (Phase 3) implements `io::PlanIo` and `io::SafetyNet` and calls
//! the routines under the journal mutex.

pub mod apply;
pub mod check;
pub mod io;
pub mod journal;
pub mod reset;
pub mod restore;

#[cfg(any(test, feature = "test-support"))]
pub mod fake;

#[cfg(test)]
mod props;
