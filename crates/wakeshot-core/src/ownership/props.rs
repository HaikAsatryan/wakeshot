//! The journal property test: no sequence of apply, user change, plan switch, failure or stop, then a restore, drops
//! an entry whose value Wakeshot still holds (test only).
//!
//! Owner: `p2-ownership-props`.
