//! The notice seam (architecture section 5, "Tray, notices, sounds"): `BalloonNotifier`.
//!
//! Owner: tray track (`p5-notices`). The trait and `FakeNotifier` are contract.

#[cfg(any(test, feature = "test-support"))]
use std::cell::RefCell;

use wakeshot_core::view::Notice;

use crate::WinError;

/// Shows one notice, so a later move from balloons to registered toasts changes only the implementation.
///
/// Owner: tray track (Phase 5). `Send`: it runs on the IO thread. The IO thread may rely on: `show` returns once
/// Windows took the notice or refused it; the result becomes `Outcome::Notified`. `Ok` never proves the user saw it
/// (architecture section 5).
pub trait Notifier: Send {
    fn show(&self, notice: &Notice) -> Result<(), WinError>;
}

/// Records each notice; `fail_with` makes every `show` fail, to test the `Notified(Err)` path.
///
/// Owner: tray track (`p5-notices`); first version from the contracts card.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct FakeNotifier {
    pub shown: RefCell<Vec<Notice>>,
    pub fail_with: Option<WinError>,
}

#[cfg(any(test, feature = "test-support"))]
impl Notifier for FakeNotifier {
    fn show(&self, notice: &Notice) -> Result<(), WinError> {
        if let Some(e) = self.fail_with {
            return Err(e);
        }
        self.shown.borrow_mut().push(notice.clone());
        Ok(())
    }
}
