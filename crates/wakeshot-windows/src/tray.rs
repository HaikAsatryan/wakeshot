//! The tray: icon, menu, theme and icon art (architecture section 5, "Tray, notices, sounds").
//!
//! `TrayIcon` owns its own hidden window on the loop thread, so no tray message reaches the loop's window.

pub mod art;
pub mod icon;
pub mod menu;
pub mod menu_paint;
pub mod theme;

use wakeshot_core::view::MenuModel;

/// Where the menu opens: the click point on screen, in physical pixels, as the tray icon's message gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub x: i32,
    pub y: i32,
}

/// The notify icon's identity (its window handle and GUID), so `BalloonNotifier` on the IO thread modifies the icon
/// `TrayIcon` owns on the loop thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IconIdentity {
    pub hwnd: isize,
    pub guid: u128,
}

/// Draws the menu, so how it looks never changes the core or the loop.
///
/// Callers may rely on: `show` draws `menu` as given and returns at once or when the menu closes; the item picked
/// comes back only as `Event::Menu(command)` through the event sink, never as a return value, so a XAML flyout with
/// its own thread fits the same seam.
pub trait MenuView {
    fn show(&mut self, menu: &MenuModel, anchor: Anchor);
}
