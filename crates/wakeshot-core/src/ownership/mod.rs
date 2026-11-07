//! Settings ownership: the one apply, the one restore routine and the ownership check every process runs
//! (architecture section 5), and the reset of a lost journal the tray runs. They are generic over the two seams in
//! `io`, so the stop-point tests drive the real order of steps. `wakeshot-windows` implements the seams and calls the
//! routines under the journal mutex.

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
