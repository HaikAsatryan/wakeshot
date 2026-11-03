//! The dispatcher: the core in the loop, the runner for each command, the tick, end of session and quit.
//!
//! Owner: `p6-int-dispatch`; fixes by the e2e cards. `Dispatcher` implements `LoopHandler`; trait `LoopRunners` holds
//! the loop's runners (`docs/phases/06-integration.md` note 4).
