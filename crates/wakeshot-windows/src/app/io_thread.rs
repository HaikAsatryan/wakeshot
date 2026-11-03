//! The IO thread: its loop over `IoQueue`, the executor, file commands, `Log`, `Notify` and `OpenLog`.
//!
//! Owner: `p6-int-io-thread`; fixes by the e2e cards. It sets the `Ack` after the end-of-session restore
//! (`docs/phases/06-integration.md` note 5).
