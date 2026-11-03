//! The loop window: the hidden top-level window, the message pump, the one timer, `WindowSink` and `ask_quit`.
//!
//! Owner: `p6-int-window`; fixes by the e2e cards. `WindowSink` is the real `EventSink`; the window calls
//! `LoopHandler` (`docs/phases/06-integration.md` note 3).
//!
//! The pump takes the handler shared: `LoopWindow::run(self, handler: &dyn LoopHandler) -> Result<(), WinError>`. A
//! nested pump calls the handler again while an outer call runs, and only shared references make that sound (the
//! `LoopHandler` doc in `app.rs`).
