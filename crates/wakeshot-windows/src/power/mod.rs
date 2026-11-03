//! Power requests, power plans, the journal, hibernate and system power events (architecture section 5).
//!
//! Owner: contracts card (this module list).

pub mod executor;
pub mod hibernate;
pub mod journal;
pub mod plan;
pub mod requests;
pub mod system;

#[cfg(test)]
mod stop_points;
