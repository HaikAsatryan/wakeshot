//! The Velopack wiring, the only file that names the `velopack` crate (architecture section 4, tray startup step 1).
//!
//! Owner: ship track (`p6-velopack-dependency`), which keeps this signature.

/// Runs `VelopackApp` with the hook callbacks; for a hook, Velopack runs it and exits the process here.
pub fn startup() {}
