//! The notice seam (architecture section 5, "Tray, notices, sounds"): `BalloonNotifier`.
//!
//! Owner: tray track (`p5-notices`), then `p6-brand-apply-art` (the notice icon). The trait and `FakeNotifier` are
//! contract.
//!
//! A notice is a balloon on Wakeshot's own tray icon: Windows 11 shows it as a banner with no app registration and
//! does not keep it in the notification centre (research, "Balloon on Windows 11"; `Shell_NotifyIconW` Remarks). The
//! balloon changes only the info fields of the icon `TrayIcon` owns, so the icon and its tooltip stay as they are.
//!
//! The banner's header (name and picture) comes from the app id Windows files the balloon under. An installed tray
//! has Velopack's id, whose key the hooks write ([`NoticeAppId`]); without that key the header showed a plain white
//! square (`docs/tasks/p6-notice-app-icon.md`).

#[cfg(any(test, feature = "test-support"))]
use std::cell::RefCell;
use std::ffi::{OsStr, c_void};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use wakeshot_core::text::{self, NoticeClick, NoticeText};
use wakeshot_core::view::Notice;
use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_INVALID_PARAMETER, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS,
    GetLastError, HWND, SetLastError, WIN32_ERROR,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE,
    RegCreateKeyExW, RegDeleteKeyW, RegDeleteTreeW, RegDeleteValueW, RegSetValueExW,
};
use windows::Win32::UI::HiDpi::GetSystemMetricsForDpi;
use windows::Win32::UI::Shell::{
    NIF_GUID, NIF_INFO, NIIF_INFO, NIIF_LARGE_ICON, NIIF_NONE, NIIF_NOSOUND, NIIF_USER,
    NIIF_WARNING, NIM_MODIFY, NOTIFY_ICON_INFOTIP_FLAGS, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, DestroyIcon, HICON, LR_DEFAULTCOLOR, SM_CXICON,
};
use windows::core::{GUID, Owned, PCWSTR};

use crate::WinError;
use crate::log::{Level, Log};
use crate::tray::icon::taskbar_dpi;
use crate::tray::{IconIdentity, art};

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

/// The large icon of a notice that is not a warning. A warning always shows Windows' warning icon, so it reads as a
/// warning whichever is picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconChoice {
    /// Wakeshot's app icon (`NIIF_USER` with `hBalloonIcon`).
    App,
    /// Windows' info icon (`NIIF_INFO`).
    Info,
}

impl IconChoice {
    /// The product's pick until `p5-notices` Part B compares both on the banner, light and dark.
    pub const PRODUCT: IconChoice = IconChoice::App;
}

/// `Shell_NotifyIconW(NIM_MODIFY)`, behind a seam so no test shows a balloon.
type Shell = dyn Fn(&NOTIFYICONDATAW) -> Result<(), WinError> + Send + Sync;

/// The wait before a refused notice is tried again. Seen once (2026-11-05 11:24:39): right after a resume and a lid
/// open, the shell refused the second of two notices sent 6 ms apart (1462, "Incorrect size argument"), while the same
/// pair 21 ms apart at an earlier resume both showed, and every other notice of two screen sessions showed at once
/// (research, "Notice retry"). So the refusal is short-lived; 1 s is well past both gaps, and is the settle time the
/// icon gives display and theme broadcasts. It holds up the IO thread only after a refusal. Change it when a log shows
/// a second refusal after the wait.
const RETRY_AFTER: Duration = Duration::from_secs(1);

const AREA: &str = "notify";

/// The product's `Notifier`: each notice as a balloon on the tray icon, with the words, warning look, sound and
/// click action of `text::notice`.
///
/// `Send + Sync`, so one `Arc` serves both threads: the IO thread shows notices through a clone (`Arc<BalloonNotifier>`
/// is a `Notifier` too), and the tray icon's balloon click hook on the loop thread calls `take_click` on another.
pub struct BalloonNotifier {
    icon: IconIdentity,
    choice: IconChoice,
    shell: Box<Shell>,
    /// Only `show` takes it, and holds it through the shell call, so two notices never mix. `take_click` never waits
    /// on it: a shell call that waits on Explorer must not hold up the loop thread.
    large: Mutex<Option<LargeIcon>>,
    click: Mutex<NoticeClick>,
    kept: Mutex<Kept>,
    retry_after: Duration,
    log: Option<Arc<dyn Log>>,
}

/// The notices the shell refused since the icon was last added, and how many adds there were.
#[derive(Default)]
struct Kept {
    adds: u64,
    notices: Vec<Notice>,
}

impl BalloonNotifier {
    /// Notices on the icon `icon` names (`TrayIcon::identity`), with the product's icon choice. The icon must be
    /// added (the first `TrayIcon::show`) before the first notice, or `show` fails and the notice waits for
    /// `icon_added`.
    pub fn new(icon: IconIdentity) -> Self {
        Self::with_choice(icon, IconChoice::PRODUCT)
    }

    /// Demos only (`ux_notices --icon`): the large icon given instead of the product's pick.
    pub fn with_choice(icon: IconIdentity, choice: IconChoice) -> Self {
        Self::with_shell(icon, choice, Box::new(shell_modify))
    }

    fn with_shell(icon: IconIdentity, choice: IconChoice, shell: Box<Shell>) -> Self {
        BalloonNotifier {
            icon,
            choice,
            shell,
            large: Mutex::new(None),
            click: Mutex::new(NoticeClick::Nothing),
            kept: Mutex::new(Kept::default()),
            retry_after: RETRY_AFTER,
            log: None,
        }
    }

    /// Logs each retry of a refused notice and how it ended. The IO thread's own line for the notice, shown or not,
    /// comes right after them and names it.
    pub fn with_log(mut self, log: Arc<dyn Log>) -> Self {
        self.log = Some(log);
        self
    }

    /// The icon was just added (`tray::icon::Note::Added`): gives back, in order, each notice the shell refused since
    /// the last add, for the caller to show again. A tray started at sign-in before the taskbar exists adds its icon
    /// late, and the first-run notice that came first must not be lost. The caller shows them through the IO thread:
    /// this never waits on a shell call, so the loop thread may call it.
    pub fn icon_added(&self) -> Vec<Notice> {
        let mut kept = lock(&self.kept);
        kept.adds += 1;
        std::mem::take(&mut kept.notices)
    }

    /// What a click on the balloon opens: the click action of the last balloon Windows took, then `Nothing`, so a
    /// second click message or a late click on an old banner opens nothing. `NIN_BALLOONUSERCLICK` does not say which
    /// balloon was clicked.
    pub fn take_click(&self) -> NoticeClick {
        std::mem::replace(&mut *lock(&self.click), NoticeClick::Nothing)
    }
}

impl Notifier for BalloonNotifier {
    /// A refused notice is tried once more after `RETRY_AFTER`, and at once when the icon was added while the call
    /// ran (that add found nothing to give back, and the icon is there now); each of the two at most once, so a shell
    /// that keeps refusing costs at most three calls. Still refused, it is kept for the next add (`icon_added`).
    fn show(&self, notice: &Notice) -> Result<(), WinError> {
        let text = text::notice(notice);
        let (mut at_once, mut waited) = (false, false);
        loop {
            let adds = lock(&self.kept).adds;
            let error = match self.call(&text) {
                Ok(()) => {
                    if at_once || waited {
                        self.note("the notice showed at its retry");
                    }
                    *lock(&self.click) = text.click;
                    return Ok(());
                }
                Err(e) => e,
            };
            let mut kept = lock(&self.kept);
            let wait = if kept.adds != adds && !at_once {
                at_once = true;
                Duration::ZERO
            } else if !waited {
                waited = true;
                self.retry_after
            } else {
                if !kept.notices.contains(notice) {
                    kept.notices.push(notice.clone());
                }
                drop(kept);
                self.note(&format!(
                    "the notice was refused again ({} failed with {}); it is shown when the icon is added again",
                    error.call, error.code
                ));
                return Err(error);
            };
            drop(kept);
            let when = if wait.is_zero() {
                "at once, as the icon was added meanwhile".to_owned()
            } else {
                format!("in {} ms", wait.as_millis())
            };
            self.note(&format!(
                "a notice was refused ({} failed with {}); trying it again {when}",
                error.call, error.code
            ));
            std::thread::sleep(wait);
        }
    }
}

impl BalloonNotifier {
    /// One shell call, with the app icon at the size of this moment: a retry after a display change draws it anew.
    fn call(&self, text: &NoticeText) -> Result<(), WinError> {
        let mut data = fill(text, self.icon);
        let mut large = lock(&self.large);
        let mut choice = self.choice;
        let mut old = None;
        if choice == IconChoice::App && !text.warning {
            let size = large_icon_size();
            if large.as_ref().is_none_or(|icon| icon.size != size)
                && let Some(icon) = LargeIcon::draw(size)
            {
                old = large.replace(icon);
            }
            // At an old size only when a new one could not be drawn: the shell scales it. With none at all (out of
            // handles), Windows' info icon still shows the notice.
            match large.as_ref() {
                Some(icon) => data.hBalloonIcon = icon.handle(),
                None => choice = IconChoice::Info,
            }
        }
        data.dwInfoFlags = info_flags(text, choice);
        let shown = (self.shell)(&data);
        // An icon of an old size goes only once the shell has the new one.
        drop(old);
        shown
    }

    /// Info: the IO thread's line after it says whether the notice showed, as a warning when it did not.
    fn note(&self, text: &str) {
        if let Some(log) = &self.log {
            log.line(Level::Info, AREA, text);
        }
    }
}

/// The IO thread owns one clone and the balloon click hook another (Phase 5 track note 5).
impl Notifier for Arc<BalloonNotifier> {
    fn show(&self, notice: &Notice) -> Result<(), WinError> {
        (**self).show(notice)
    }
}

/// The balloon fields of the notify icon `icon` names: title and body cut to fit on a character boundary, and no
/// `NIF_ICON` or `NIF_TIP`, so the icon and its tooltip stay as `TrayIcon` set them. `dwInfoFlags` and
/// `hBalloonIcon` are left to the caller.
fn fill(text: &NoticeText, icon: IconIdentity) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(icon.hwnd as *mut c_void),
        uFlags: NIF_INFO | NIF_GUID,
        guidItem: GUID::from_u128(icon.guid),
        ..Default::default()
    };
    // `text::notice` already fits both (and keeps them non-empty: an empty `szInfo` removes a balloon instead of
    // showing one); this is the last guard.
    put_wide(&mut data.szInfoTitle, &text.title);
    put_wide(&mut data.szInfo, &text.body);
    data
}

/// The balloon's icon and sound. `NIIF_LARGE_ICON` asks for the `SM_CXICON` size; the page allows it with the stock
/// icons too. Never `NIIF_RESPECT_QUIET_TIME`: a notice Windows drops in the first hour after a user's first sign-in
/// is lost without a trace, and each notice tells of something Wakeshot did or could not do (decision 0).
fn info_flags(text: &NoticeText, choice: IconChoice) -> NOTIFY_ICON_INFOTIP_FLAGS {
    let icon = match (text.warning, choice) {
        (true, _) => NIIF_WARNING,
        (false, IconChoice::App) => NIIF_USER,
        (false, IconChoice::Info) => NIIF_INFO,
    };
    // A notice that comes with a chime is silent, so two sounds never overlap (track note 3.5).
    let sound = if text.silent { NIIF_NOSOUND } else { NIIF_NONE };
    icon | NIIF_LARGE_ICON | sound
}

/// `text` into a NUL-terminated UTF-16 buffer, cut on a character boundary so no half of a surrogate pair is shown.
fn put_wide(buf: &mut [u16], text: &str) {
    let room = buf.len().saturating_sub(1);
    let mut used = 0;
    for c in text.chars() {
        let mut units = [0u16; 2];
        let units = c.encode_utf16(&mut units);
        let end = used + units.len();
        if end > room {
            break;
        }
        buf[used..end].copy_from_slice(units);
        used = end;
    }
    if let Some(end) = buf.get_mut(used) {
        *end = 0;
    }
}

fn shell_modify(data: &NOTIFYICONDATAW) -> Result<(), WinError> {
    // The page does not say FALSE sets a last error; cleared first, an old code never shows up as this one's, and 0
    // means Windows gave none.
    // SAFETY: plain calls; `data` is a live, fully initialised NOTIFYICONDATAW with `cbSize` set.
    unsafe {
        SetLastError(WIN32_ERROR(0));
        if Shell_NotifyIconW(NIM_MODIFY, data).as_bool() {
            Ok(())
        } else {
            Err(WinError {
                call: "Shell_NotifyIconW",
                code: GetLastError().0,
            })
        }
    }
}

/// The app icon for the banner. Kept until the notifier goes (or the size changes), since neither page says whether
/// the shell copies `hBalloonIcon`, and a queued balloon may draw after the call returns. Held as `isize` so the
/// notifier stays `Send + Sync`.
struct LargeIcon {
    size: u32,
    handle: isize,
}

impl LargeIcon {
    /// `None` when Windows has no handles left; the notice then shows with the info icon.
    fn draw(size: u32) -> Option<Self> {
        // Through the ICO writer `art` already tests: one frame of it is the 32-bit DIB with an AND mask that
        // `CreateIconFromResourceEx` reads, as an icon resource holds it.
        let file = art::ico(&[art::app_icon(size)]);
        let frame = first_frame(&file)?;
        let side = i32::try_from(size).ok()?;
        // SAFETY: `frame` is one whole icon image; 0x00030000 is the format version the page asks for.
        let icon = unsafe {
            CreateIconFromResourceEx(frame, true, 0x0003_0000, side, side, LR_DEFAULTCOLOR)
        }
        .ok()?;
        Some(LargeIcon {
            size,
            handle: icon.0 as isize,
        })
    }

    fn handle(&self) -> HICON {
        HICON(self.handle as *mut c_void)
    }
}

impl Drop for LargeIcon {
    fn drop(&mut self) {
        // SAFETY: made by `CreateIconFromResourceEx` and owned only here.
        let _ = unsafe { DestroyIcon(self.handle()) };
    }
}

/// The first image of an ICO file, by its directory entry (bytes in it, then where it starts).
fn first_frame(file: &[u8]) -> Option<&[u8]> {
    let entry = file.get(6..22)?;
    let len = u32::from_le_bytes(entry[8..12].try_into().ok()?) as usize;
    let at = u32::from_le_bytes(entry[12..16].try_into().ok()?) as usize;
    file.get(at..at.checked_add(len)?)
}

/// `SM_CXICON` at the taskbar's DPI, the size `NIIF_LARGE_ICON` names. Kept within the ICO writer's 16 to 256.
fn large_icon_size() -> u32 {
    let dpi = taskbar_dpi();
    // SAFETY: plain call.
    let px = unsafe { GetSystemMetricsForDpi(SM_CXICON, dpi) };
    let px = match u32::try_from(px) {
        Ok(px) if px > 0 => px,
        // 32 px at 96 dpi, rounded as `MulDiv` does.
        _ => (32 * dpi + 48) / 96,
    };
    px.clamp(16, 256)
}

/// A poisoned lock only means a panic elsewhere, and the panic hook aborts in every profile; the data is still whole.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The app id Windows files the installed tray's balloons under. `VelopackApp::run` gives every start of an installed
/// exe this explicit id, from the Start menu, the Run value or a restart after an update: "velopack." and the pack id,
/// the id of its Start menu shortcut too (velopack 1.2.161 `app.rs` and `locator.rs`; research "Velopack checks for
/// Wakeshot", AUMID). It changes only if `pack.ps1` names an AUMID or Velopack changes its rule; `packaging/e2e.ps1`
/// checks the installed shortcut's id against it. A build run from `target\` has no explicit id, so the shell files
/// its balloons under its own `NotifyIconGeneratedAumid_<id>` and this key plays no part.
pub const NOTICE_APP_ID: &str = "velopack.HaikAsatryan.Wakeshot";

/// The picture of the notice header: the app icon as a PNG, which `packaging/pack.ps1` ships beside the tray exe, so
/// Velopack puts it in place at each install and update and takes it away at uninstall.
pub const NOTICE_ICON_FILE: &str = "wakeshot-notice.png";

/// The PNG's side in pixels. Microsoft names no size for `IconUri`; the header draws the picture at a small icon's
/// size (16 px at 100%, so 24 at 150% and 32 at 200%), and 48 scales down to the two most common laptop scales by a
/// whole factor. A design call; change it when the header looks soft at some scale.
pub const NOTICE_ICON_SIZE: u32 = 48;

/// The bytes of `assets/wakeshot-notice.png`, which `tray::art::write_art_files` writes: the app icon drawn flat.
/// Scaled down to the header's 16 to 24 px, the full drawing's glow and bloom blurred the light into the plate and its
/// shadow left a grey rim on the light notice; the flat drawing keeps a clear light (`docs/research.md`, "Brand
/// apply: the held light in art.rs").
#[cfg(test)]
pub(crate) fn notice_icon_png() -> Vec<u8> {
    art::png(&art::app_icon_flat(NOTICE_ICON_SIZE))
}

/// The name in the notice header.
pub const NOTICE_NAME: &str = "Wakeshot";

/// `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot`: the name and picture Windows shows in the
/// header of each notice filed under [`NOTICE_APP_ID`]. Without it the header showed a plain white square
/// (`p6-int-haik-checks` finding 5): Windows' default picture for an id with no `IconUri` (Windows App SDK
/// `AppNotificationUtility.cpp`, "the OS will render a default icon"). The hooks are its one writer: install and
/// updated write it, uninstall removes it.
///
/// The values are the two Windows itself writes for the shell's generated balloon ids and that the Windows App SDK
/// writes for unpackaged apps, both `REG_EXPAND_SZ` as Microsoft's page ("Send a local toast notification from other
/// types of unpackaged apps", now removed from Learn) shows them. Its `IconBackgroundColor` and `CustomActivator` are
/// left out: neither of those writers sets a background colour, and a balloon click reaches the tray's own window,
/// not a COM activator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoticeAppId {
    key: String,
}

impl NoticeAppId {
    pub fn real() -> Self {
        Self::at(r"Software\Classes\AppUserModelId")
    }

    /// `base` is a key under `HKCU`; the app id's key is its subkey.
    pub fn at(base: &str) -> Self {
        Self {
            key: format!(r"{base}\{NOTICE_APP_ID}"),
        }
    }

    /// Writes `DisplayName` and, given an icon, `IconUri` with its path; the key is made when it is missing. With no
    /// icon an `IconUri` left from before is deleted, so the key never names a picture that is gone.
    pub fn register(&self, icon: Option<&Path>) -> Result<(), WinError> {
        let key = create_key(&self.key)?;
        set_expand(&key, "DisplayName", OsStr::new(NOTICE_NAME))?;
        match icon {
            Some(path) => set_expand(&key, "IconUri", path.as_os_str()),
            None => delete_value(&key, "IconUri"),
        }
    }

    /// The key with whatever is in it, Windows' own additions too. `true`: it existed and is gone now.
    pub fn remove(&self) -> Result<bool, WinError> {
        let key = wide("RegDeleteTreeW", OsStr::new(&self.key))?;
        // SAFETY: `key` is NUL-terminated and lives for the call.
        let tree = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr())) };
        if missing(tree) {
            return Ok(false);
        }
        reg_ok("RegDeleteTreeW", tree)?;
        // The emptied key itself, should the tree delete leave it, as `store`'s scratch keys assume too.
        // SAFETY: as above.
        let gone = unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr())) };
        if !missing(gone) {
            reg_ok("RegDeleteKeyW", gone)?;
        }
        Ok(true)
    }
}

fn create_key(path: &str) -> Result<Owned<HKEY>, WinError> {
    let name = wide("RegCreateKeyExW", OsStr::new(path))?;
    let mut key = HKEY::default();
    // SAFETY: `name` is NUL-terminated and lives for the call; the key is written to a live local.
    let created = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(name.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &raw mut key,
            None,
        )
    };
    reg_ok("RegCreateKeyExW", created)?;
    // SAFETY: the key is new and owned only here.
    Ok(unsafe { Owned::new(key) })
}

fn set_expand(key: &Owned<HKEY>, name: &str, value: &OsStr) -> Result<(), WinError> {
    let name = wide("RegSetValueExW", OsStr::new(name))?;
    let data: Vec<u8> = wide("RegSetValueExW", value)?
        .iter()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();
    // SAFETY: an open key with `KEY_SET_VALUE`; `name` is NUL-terminated, `data` holds the terminator too.
    let set = unsafe {
        RegSetValueExW(
            **key,
            PCWSTR(name.as_ptr()),
            None,
            REG_EXPAND_SZ,
            Some(data.as_slice()),
        )
    };
    reg_ok("RegSetValueExW", set)
}

/// A missing value is not an error.
fn delete_value(key: &Owned<HKEY>, name: &str) -> Result<(), WinError> {
    let name = wide("RegDeleteValueW", OsStr::new(name))?;
    // SAFETY: an open key with `KEY_SET_VALUE`; `name` is NUL-terminated and lives for the call.
    let deleted = unsafe { RegDeleteValueW(**key, PCWSTR(name.as_ptr())) };
    if missing(deleted) {
        return Ok(());
    }
    reg_ok("RegDeleteValueW", deleted)
}

/// NUL-terminated UTF-16. An inner NUL would cut the text short and name another key or file, so it is refused.
fn wide(call: &'static str, text: &OsStr) -> Result<Vec<u16>, WinError> {
    let mut units: Vec<u16> = text.encode_wide().collect();
    if units.contains(&0) {
        return Err(WinError {
            call,
            code: ERROR_INVALID_PARAMETER.0,
        });
    }
    units.push(0);
    Ok(units)
}

fn missing(result: WIN32_ERROR) -> bool {
    result == ERROR_FILE_NOT_FOUND || result == ERROR_PATH_NOT_FOUND
}

fn reg_ok(call: &'static str, result: WIN32_ERROR) -> Result<(), WinError> {
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(WinError {
            call,
            code: result.0,
        })
    }
}

/// The key's values read back, for tests only.
#[cfg(test)]
impl NoticeAppId {
    /// The value's type and text, unexpanded; `None`: no such value, or no such key.
    pub(crate) fn value(
        &self,
        name: &str,
    ) -> Result<Option<(windows::Win32::System::Registry::REG_VALUE_TYPE, String)>, WinError> {
        use windows::Win32::System::Registry::{
            REG_VALUE_TYPE, RRF_NOEXPAND, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ, RegGetValueW,
        };
        let key = wide("RegGetValueW", OsStr::new(&self.key))?;
        let name = wide("RegGetValueW", OsStr::new(name))?;
        let mut kind = REG_VALUE_TYPE::default();
        let mut buf = vec![0u16; 1024];
        let mut size = u32::try_from(buf.len() * 2).unwrap_or(u32::MAX);
        // SAFETY: names are NUL-terminated and live for the call; `buf` holds `size` bytes.
        let read = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | RRF_NOEXPAND,
                Some(&raw mut kind),
                Some(buf.as_mut_ptr().cast()),
                Some(&raw mut size),
            )
        };
        if missing(read) {
            return Ok(None);
        }
        reg_ok("RegGetValueW", read)?;
        buf.truncate(size as usize / 2);
        while buf.last() == Some(&0) {
            buf.pop();
        }
        Ok(Some((kind, String::from_utf16_lossy(&buf))))
    }

    /// Whether the key itself exists.
    pub(crate) fn exists(&self) -> bool {
        use windows::Win32::System::Registry::{KEY_READ, RegOpenKeyExW};
        let Ok(key) = wide("RegOpenKeyExW", OsStr::new(&self.key)) else {
            return false;
        };
        let mut opened = HKEY::default();
        // SAFETY: `key` is NUL-terminated and lives for the call; the key is written to a live local.
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                None,
                KEY_READ,
                &raw mut opened,
            )
        };
        if result == ERROR_SUCCESS {
            // SAFETY: the key is new and owned only here; it closes at once.
            drop(unsafe { Owned::new(opened) });
        }
        result == ERROR_SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{OnceLock, Weak};
    use std::time::Instant;

    use super::*;
    use crate::log::MemoryLog;
    use crate::tray::icon::IconKind;
    use wakeshot_core::settings::SettingKey;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDIBits, GetObjectW,
    };
    use windows::Win32::UI::Shell::NOTIFY_ICON_DATA_FLAGS;
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    const ID: IconIdentity = IconIdentity {
        hwnd: 0x1234,
        guid: IconKind::Demo.guid(),
    };

    fn read(buf: &[u16]) -> String {
        let end = buf.iter().position(|&u| u == 0).expect("an end mark");
        String::from_utf16(&buf[..end]).expect("whole characters")
    }

    fn text(title: &str, body: &str, warning: bool, silent: bool) -> NoticeText {
        NoticeText {
            title: title.to_owned(),
            body: body.to_owned(),
            warning,
            silent,
            click: NoticeClick::Nothing,
        }
    }

    /// What the fake shell got, in plain values: the struct holds raw handles, which do not cross threads.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Sent {
        flags: NOTIFY_ICON_DATA_FLAGS,
        info: NOTIFY_ICON_INFOTIP_FLAGS,
        title: String,
        guid: u128,
        balloon_icon: isize,
    }

    #[derive(Clone, Default)]
    struct FakeShell {
        sent: Arc<Mutex<Vec<Sent>>>,
        fail: Arc<Mutex<Option<WinError>>>,
    }

    impl FakeShell {
        /// With no wait before a retry, so a refusal costs a test no time; `retries_*` tests set their own.
        fn notifier(&self, choice: IconChoice) -> BalloonNotifier {
            let me = self.clone();
            let mut n = BalloonNotifier::with_shell(
                ID,
                choice,
                Box::new(move |data| {
                    // A retry loop fails the test instead of hanging it.
                    assert!(me.sent.lock().unwrap().len() < 100, "a retry loop");
                    me.sent.lock().unwrap().push(Sent {
                        flags: data.uFlags,
                        info: data.dwInfoFlags,
                        title: read(&data.szInfoTitle),
                        guid: data.guidItem.to_u128(),
                        balloon_icon: data.hBalloonIcon.0 as isize,
                    });
                    me.fail.lock().unwrap().map_or(Ok(()), Err)
                }),
            );
            n.retry_after = Duration::ZERO;
            n
        }

        fn sent(&self) -> Vec<Sent> {
            self.sent.lock().unwrap().clone()
        }
    }

    #[test]
    fn fills_title_and_body_within_limits() {
        let short = fill(&text("Title", "Body\nline", false, false), ID);
        assert_eq!(read(&short.szInfoTitle), "Title");
        assert_eq!(read(&short.szInfo), "Body\nline");

        // A surrogate pair that would end past the 63 units of the title is left out whole.
        let title = format!("{}\u{1F600}", "a".repeat(62));
        let body = format!("{}\u{1F600}", "b".repeat(254));
        let cut = fill(&text(&title, &body, false, false), ID);
        assert_eq!(read(&cut.szInfoTitle), "a".repeat(62));
        assert_eq!(read(&cut.szInfo), "b".repeat(254));

        // Exactly at the limits: nothing is cut.
        let title = format!("{}\u{1F600}", "a".repeat(61));
        let body = "b".repeat(255);
        let full = fill(&text(&title, &body, false, false), ID);
        assert_eq!(read(&full.szInfoTitle), title);
        assert_eq!(read(&full.szInfo), body);

        // Armenian, one unit a character.
        let long = fill(&text(&"Ա".repeat(100), &"Բ".repeat(300), false, false), ID);
        assert_eq!(read(&long.szInfoTitle), "Ա".repeat(63));
        assert_eq!(read(&long.szInfo), "Բ".repeat(255));
    }

    #[test]
    fn warning_and_silent_set_the_flags() {
        let table = [
            (false, false, IconChoice::App, NIIF_USER),
            (false, false, IconChoice::Info, NIIF_INFO),
            (true, false, IconChoice::App, NIIF_WARNING),
            (true, false, IconChoice::Info, NIIF_WARNING),
            (false, true, IconChoice::App, NIIF_USER | NIIF_NOSOUND),
            (false, true, IconChoice::Info, NIIF_INFO | NIIF_NOSOUND),
            (true, true, IconChoice::App, NIIF_WARNING | NIIF_NOSOUND),
        ];
        for (warning, silent, choice, want) in table {
            let got = info_flags(&text("t", "b", warning, silent), choice);
            assert_eq!(got, want | NIIF_LARGE_ICON, "{warning} {silent} {choice:?}");
        }
        // Every real notice: its sound follows `silent`, its icon `warning`.
        for notice in [
            Notice::StepBack { floor_pct: 20 },
            Notice::IdleLocked,
            Notice::FirstRun,
            Notice::RestoreFailed,
        ] {
            let t = text::notice(&notice);
            let flags = info_flags(&t, IconChoice::Info);
            assert_eq!(flags.contains(NIIF_NOSOUND), t.silent, "{notice:?}");
            assert_eq!(flags.0 & 0xf == NIIF_WARNING.0, t.warning, "{notice:?}");
        }
    }

    #[test]
    fn uses_the_guid_identity() {
        let data = fill(&text("t", "b", true, true), ID);
        assert_eq!(data.cbSize as usize, size_of::<NOTIFYICONDATAW>());
        assert_eq!(data.hWnd.0 as isize, ID.hwnd);
        assert_eq!(data.guidItem.to_u128(), IconKind::Demo.guid());
        // Only the balloon: the icon, tooltip and callback stay as `TrayIcon` set them.
        assert_eq!(data.uFlags, NIF_INFO | NIF_GUID);
        assert!(data.hIcon.is_invalid());
        assert_eq!(data.szTip[0], 0);
        assert_eq!(data.dwInfoFlags, NIIF_NONE, "set by the caller");
        assert!(data.hBalloonIcon.is_invalid(), "set by the caller");
    }

    #[test]
    fn click_is_kept_for_the_last_notice_only() {
        let shell = FakeShell::default();
        let n = shell.notifier(IconChoice::Info);
        n.show(&Notice::FirstRun).unwrap();
        n.show(&Notice::RestoreFailed).unwrap();
        assert_eq!(n.take_click(), NoticeClick::OpenLog);

        // A notice with no click action clears the one before.
        n.show(&Notice::RestoreFailed).unwrap();
        n.show(&Notice::IdleLocked).unwrap();
        assert_eq!(n.take_click(), NoticeClick::Nothing);

        // A notice Windows refused never showed, so the click stays with the one that did.
        n.show(&Notice::JournalLost {
            reset: Vec::new(),
            failed: true,
        })
        .unwrap();
        let refused = WinError {
            call: "Shell_NotifyIconW",
            code: 0,
        };
        *shell.fail.lock().unwrap() = Some(refused);
        assert_eq!(n.show(&Notice::FirstRun), Err(refused));
        assert_eq!(n.take_click(), NoticeClick::OpenLog);
        assert_eq!(
            shell.sent().len(),
            7,
            "every notice went to the shell, the refused one twice"
        );
    }

    /// What the shell gives for a balloon on an icon not added yet; the page names no code, so 0.
    const NO_ICON: WinError = WinError {
        call: "Shell_NotifyIconW",
        code: 0,
    };

    #[test]
    fn a_refused_notice_waits_for_the_next_add() {
        let shell = FakeShell::default();
        let n = shell.notifier(IconChoice::Info);
        *shell.fail.lock().unwrap() = Some(NO_ICON);
        assert_eq!(n.show(&Notice::FirstRun), Err(NO_ICON));
        assert_eq!(n.show(&Notice::FirstRun), Err(NO_ICON));
        assert_eq!(n.show(&Notice::RestoreFailed), Err(NO_ICON));
        assert_eq!(n.take_click(), NoticeClick::Nothing, "none showed");

        *shell.fail.lock().unwrap() = None;
        assert_eq!(
            n.icon_added(),
            [Notice::FirstRun, Notice::RestoreFailed],
            "in order, each once"
        );
        assert_eq!(n.icon_added(), [], "given back once");
        n.show(&Notice::FirstRun).unwrap();
        assert_eq!(n.take_click(), NoticeClick::OpenTaskbarSettings);
        assert_eq!(
            shell.sent().len(),
            3 * 2 + 1,
            "each refused show tried twice, no call of its own at the add"
        );
    }

    #[test]
    fn an_add_during_the_shell_call_tries_the_notice_again_at_once() {
        let slot: Arc<OnceLock<Weak<BalloonNotifier>>> = Arc::default();
        let calls = Arc::new(AtomicU32::new(0));
        let (me, count) = (Arc::clone(&slot), Arc::clone(&calls));
        let n = Arc::new(BalloonNotifier::with_shell(
            ID,
            IconChoice::Info,
            Box::new(move |_| {
                if count.fetch_add(1, Ordering::SeqCst) > 0 {
                    return Ok(());
                }
                // The loop thread adds the icon while this first call waits on the shell.
                let n = me.get().and_then(Weak::upgrade).expect("the notifier");
                assert_eq!(n.icon_added(), [], "not refused yet");
                Err(NO_ICON)
            }),
        ));
        slot.set(Arc::downgrade(&n)).unwrap();

        let started = Instant::now();
        n.show(&Notice::FirstRun).unwrap();
        assert!(started.elapsed() < RETRY_AFTER, "no wait before this retry");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(n.icon_added(), [], "it showed, so nothing waits");
        assert_eq!(n.take_click(), NoticeClick::OpenTaskbarSettings);
    }

    /// The code the shell gave at the lost closed-lid report (`p6-screen-reruns`, 2026-11-05 11:24:39).
    const REFUSED: WinError = WinError {
        call: "Shell_NotifyIconW",
        code: 1462,
    };

    /// A notifier whose shell refuses the first `refusals` calls; it counts every call.
    fn refusing(
        refusals: u32,
        wait: Duration,
    ) -> (BalloonNotifier, Arc<AtomicU32>, Arc<MemoryLog>) {
        let calls = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&calls);
        let log = Arc::new(MemoryLog::default());
        let mut n = BalloonNotifier::with_shell(
            ID,
            IconChoice::Info,
            Box::new(move |_| {
                if count.fetch_add(1, Ordering::SeqCst) < refusals {
                    Err(REFUSED)
                } else {
                    Ok(())
                }
            }),
        )
        .with_log(Arc::clone(&log) as Arc<dyn Log>);
        n.retry_after = wait;
        (n, calls, log)
    }

    fn lines(log: &MemoryLog) -> Vec<String> {
        log.lines
            .lock()
            .unwrap()
            .iter()
            .map(|(level, area, text)| {
                assert_eq!((*level, *area), (Level::Info, AREA), "{text}");
                text.clone()
            })
            .collect()
    }

    #[test]
    fn retries_a_refused_notice_once_after_the_wait() {
        let wait = Duration::from_millis(60);
        let (n, calls, log) = refusing(1, wait);
        let started = Instant::now();
        n.show(&Notice::RestoreFailed).unwrap();
        assert!(started.elapsed() >= wait, "{:?}", started.elapsed());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            lines(&log),
            [
                "a notice was refused (Shell_NotifyIconW failed with 1462); trying it again in 60 ms",
                "the notice showed at its retry",
            ]
        );
        assert_eq!(n.icon_added(), [], "it showed, so nothing waits");
        assert_eq!(n.take_click(), NoticeClick::OpenLog);
    }

    /// More refusals than two shows may ask for: a retry loop ends in a call that works and fails the test, not hangs.
    const MANY: u32 = 10;

    #[test]
    fn retries_a_notice_refused_twice_no_more_and_keeps_it() {
        let (n, calls, log) = refusing(MANY, Duration::from_millis(10));
        assert_eq!(n.show(&Notice::FirstRun), Err(REFUSED));
        assert_eq!(calls.load(Ordering::SeqCst), 2, "one retry, no loop");
        assert_eq!(
            lines(&log),
            [
                "a notice was refused (Shell_NotifyIconW failed with 1462); trying it again in 10 ms",
                "the notice was refused again (Shell_NotifyIconW failed with 1462); it is shown when the icon is \
                 added again",
            ]
        );
        assert_eq!(n.take_click(), NoticeClick::Nothing, "none showed");
        assert_eq!(n.icon_added(), [Notice::FirstRun]);

        // The next notice gets its own retry: nothing is left from the one before.
        assert_eq!(n.show(&Notice::FirstRun), Err(REFUSED));
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn retries_at_most_three_calls_when_the_icon_is_added_during_each() {
        let slot: Arc<OnceLock<Weak<BalloonNotifier>>> = Arc::default();
        let calls = Arc::new(AtomicU32::new(0));
        let (me, count) = (Arc::clone(&slot), Arc::clone(&calls));
        let mut n = BalloonNotifier::with_shell(
            ID,
            IconChoice::Info,
            Box::new(move |_| {
                if count.fetch_add(1, Ordering::SeqCst) >= MANY {
                    return Ok(());
                }
                // Taskbar restarts, one during each call, and still every call is refused.
                let n = me.get().and_then(Weak::upgrade).expect("the notifier");
                let _ = n.icon_added();
                Err(REFUSED)
            }),
        );
        n.retry_after = Duration::from_millis(10);
        let n = Arc::new(n);
        slot.set(Arc::downgrade(&n)).unwrap();

        assert_eq!(n.show(&Notice::IdleLocked), Err(REFUSED));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(
            n.icon_added(),
            [Notice::IdleLocked],
            "kept for the next add"
        );
    }

    #[test]
    fn retry_wait_is_one_second() {
        let n = BalloonNotifier::with_shell(ID, IconChoice::Info, Box::new(|_| Ok(())));
        assert_eq!(n.retry_after, Duration::from_secs(1));
    }

    #[test]
    fn take_click_clears_it() {
        let shell = FakeShell::default();
        let n = shell.notifier(IconChoice::App);
        assert_eq!(n.take_click(), NoticeClick::Nothing, "no notice yet");
        n.show(&Notice::FirstRun).unwrap();
        assert_eq!(n.take_click(), NoticeClick::OpenTaskbarSettings);
        assert_eq!(n.take_click(), NoticeClick::Nothing);
    }

    #[test]
    fn notifier_is_send() {
        fn send_sync<T: Send + Sync + 'static>() {}
        fn notifier<T: Notifier + 'static>() {}
        send_sync::<BalloonNotifier>();
        notifier::<BalloonNotifier>();
        notifier::<Arc<BalloonNotifier>>();
    }

    #[test]
    fn show_sends_the_words_flags_and_app_icon() {
        let shell = FakeShell::default();
        let n = shell.notifier(IconChoice::App);
        n.show(&Notice::FirstRun).unwrap();
        n.show(&Notice::Updated {
            version: "1.1.0".to_owned(),
        })
        .unwrap();
        n.show(&Notice::RestoreFailed).unwrap();
        let sent = shell.sent();
        assert!(
            sent.iter()
                .all(|s| s.flags == NIF_INFO | NIF_GUID && s.guid == ID.guid)
        );
        assert_eq!(sent[0].title, text::notice(&Notice::FirstRun).title);
        assert_eq!(sent[0].info, NIIF_USER | NIIF_LARGE_ICON);
        assert_ne!(sent[0].balloon_icon, 0, "the app icon");
        assert_eq!(
            sent[1].balloon_icon, sent[0].balloon_icon,
            "drawn once per size"
        );
        assert_eq!(sent[1].info, NIIF_USER | NIIF_LARGE_ICON | NIIF_NOSOUND);
        assert_eq!(sent[2].info, NIIF_WARNING | NIIF_LARGE_ICON);
        assert_eq!(sent[2].balloon_icon, 0, "a warning has no app icon");

        let shell = FakeShell::default();
        let n = shell.notifier(IconChoice::Info);
        n.show(&Notice::UserChanged(SettingKey::LidAc)).unwrap();
        assert_eq!(shell.sent()[0].info, NIIF_INFO | NIIF_LARGE_ICON);
        assert_eq!(shell.sent()[0].balloon_icon, 0);
    }

    #[test]
    fn first_frame_of_one_frame_ico() {
        let file = art::ico(&[art::app_icon(16)]);
        let frame = first_frame(&file).expect("a frame");
        // A BITMAPINFOHEADER, 16 rows of 16 BGRA pixels, 16 mask rows of 32 bits.
        assert_eq!(frame.len(), 40 + 16 * 16 * 4 + 16 * 4);
        assert_eq!(frame[..4], 40u32.to_le_bytes());
        assert_eq!(first_frame(&file[..21]), None);
        assert_eq!(first_frame(&file[..file.len() - 1]), None);
    }

    #[test]
    fn large_icon_reads_back() {
        let size = 48;
        let icon = LargeIcon::draw(size).expect("icon");
        let mut info = ICONINFO::default();
        // SAFETY: a live icon and a valid out pointer; the bitmaps it returns are freed below.
        unsafe { GetIconInfo(icon.handle(), &mut info) }.expect("GetIconInfo");
        // SAFETY: both bitmaps are ours to free once read.
        let free = |b: windows::Win32::Graphics::Gdi::HBITMAP| unsafe {
            let _ = DeleteObject(b.into());
        };
        let mut bm = BITMAP::default();
        // SAFETY: a live bitmap and a BITMAP-sized out buffer.
        let n = unsafe {
            GetObjectW(
                info.hbmColor.into(),
                size_of::<BITMAP>() as i32,
                Some((&raw mut bm).cast()),
            )
        };
        assert!(n > 0);
        assert_eq!((bm.bmWidth, bm.bmHeight), (size as i32, size as i32));
        assert_eq!(bm.bmBitsPixel, 32);

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size as i32,
                biHeight: -(size as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut px = vec![0u8; (size * size * 4) as usize];
        // SAFETY: a memory DC made and freed here; `px` holds `size` rows of 32-bit pixels as `bmi` says.
        let rows = unsafe {
            let dc = CreateCompatibleDC(None);
            let rows = GetDIBits(
                dc,
                info.hbmColor,
                0,
                size,
                Some(px.as_mut_ptr().cast()),
                &mut bmi,
                DIB_RGB_COLORS,
            );
            let _ = DeleteDC(dc);
            rows
        };
        free(info.hbmColor);
        free(info.hbmMask);
        assert_eq!(rows, size as i32);
        // The centre of the disc and a clear corner, as the art draws them.
        let art = art::app_icon(size);
        let centre = ((size / 2 * size + size / 2) * 4) as usize;
        let want = &art.pixels[centre..centre + 4];
        assert_eq!(
            px[centre..centre + 4],
            [want[2], want[1], want[0], want[3]],
            "BGRA, alpha kept"
        );
        assert_eq!(px[3], 0, "the corner is clear");
    }

    #[test]
    fn large_icon_size_is_the_large_metric() {
        let size = large_icon_size();
        // 32 px at 100% to 128 px at 400%.
        assert!((32..=128).contains(&size), "{size}");
        assert_eq!(size % 8, 0, "{size}");
    }

    use windows::Win32::System::Registry::REG_VALUE_TYPE;

    use crate::store::ScratchKey;

    fn expand(text: &str) -> Result<Option<(REG_VALUE_TYPE, String)>, WinError> {
        Ok(Some((REG_EXPAND_SZ, text.to_owned())))
    }

    #[test]
    fn app_id_key_is_written_and_removed() {
        let scratch = ScratchKey::new("notify::tests::app_id_key_is_written_and_removed");
        let id = NoticeAppId::at(scratch.path());
        assert!(!id.exists());
        assert_eq!(id.remove(), Ok(false), "nothing to remove yet");

        let icon = Path::new(
            r"C:\Users\Ա Բ\AppData\Local\HaikAsatryan.Wakeshot\current\wakeshot-notice.png",
        );
        id.register(Some(icon)).unwrap();
        assert_eq!(id.value("DisplayName"), expand("Wakeshot"));
        assert_eq!(id.value("IconUri"), expand(&icon.display().to_string()));
        // Each install and update writes it again.
        id.register(Some(icon)).unwrap();
        assert_eq!(id.value("IconUri"), expand(&icon.display().to_string()));

        // No icon file: the name stays and the old picture goes.
        id.register(None).unwrap();
        assert_eq!(id.value("DisplayName"), expand("Wakeshot"));
        assert_eq!(id.value("IconUri"), Ok(None));

        // What Windows may add under the key goes with it.
        let extra = create_key(&format!(r"{}\Extra", id.key)).unwrap();
        set_expand(&extra, "Some", OsStr::new("value")).unwrap();
        drop(extra);
        assert_eq!(id.remove(), Ok(true));
        assert!(!id.exists());
        assert_eq!(id.remove(), Ok(false), "a second uninstall finds nothing");
    }

    #[test]
    fn app_id_key_refuses_an_inner_nul() {
        let scratch = ScratchKey::new("notify::tests::app_id_key_refuses_an_inner_nul");
        let id = NoticeAppId::at(scratch.path());
        let bad = Path::new("C:\\x\0y.png");
        assert_eq!(
            id.register(Some(bad)),
            Err(WinError {
                call: "RegSetValueExW",
                code: ERROR_INVALID_PARAMETER.0,
            })
        );
        assert_eq!(id.value("IconUri"), Ok(None));
    }

    #[test]
    fn real_app_id_key_is_under_the_classes_key() {
        assert_eq!(
            NoticeAppId::real().key,
            r"Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot"
        );
    }

    #[test]
    fn notice_icon_file_matches_the_art() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets")
            .join(NOTICE_ICON_FILE);
        let file = std::fs::read(path).expect("assets/wakeshot-notice.png");
        let side = |at: usize| u32::from_be_bytes(file[at..at + 4].try_into().unwrap());
        assert_eq!((side(16), side(20)), (NOTICE_ICON_SIZE, NOTICE_ICON_SIZE));
        // Exact: the art uses no sin, cos or exp and the PNG writer has no randomness, so x64 and ARM64 write the same
        // bytes.
        assert!(
            file == notice_icon_png(),
            "assets/{NOTICE_ICON_FILE} differs from the art: run `cargo test --locked -p wakeshot-windows \
             tray::art::write_art_files -- --ignored`"
        );
    }
}
