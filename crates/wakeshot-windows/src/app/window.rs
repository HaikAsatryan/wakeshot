//! The loop window: the hidden top-level window, the message pump, the one timer, `WindowSink` and `ask_quit`.
//!
//! Owner: `p6-int-window`; fixes by the e2e cards. `WindowSink` is the real `EventSink`; the window calls
//! `LoopHandler` (`docs/phases/06-integration.md` note 3).
