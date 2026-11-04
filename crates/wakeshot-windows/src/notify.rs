//! The notice seam (architecture section 5, "Tray, notices, sounds"): `BalloonNotifier`.
//!
//! Owner: tray track (`p5-notices`). The trait and `FakeNotifier` are contract.
//!
//! A notice is a balloon on Wakeshot's own tray icon: Windows 11 shows it as a banner with no app registration and
//! does not keep it in the notification centre (research, "Balloon on Windows 11"; `Shell_NotifyIconW` Remarks). The
//! balloon changes only the info fields of the icon `TrayIcon` owns, so the icon and its tooltip stay as they are.

#[cfg(any(test, feature = "test-support"))]
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use wakeshot_core::text::{self, NoticeClick, NoticeText};
use wakeshot_core::view::Notice;
use windows::Win32::Foundation::{GetLastError, HWND, SetLastError, WIN32_ERROR};
use windows::Win32::UI::HiDpi::GetSystemMetricsForDpi;
use windows::Win32::UI::Shell::{
    NIF_GUID, NIF_INFO, NIIF_INFO, NIIF_LARGE_ICON, NIIF_NONE, NIIF_NOSOUND, NIIF_USER,
    NIIF_WARNING, NIM_MODIFY, NOTIFY_ICON_INFOTIP_FLAGS, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, DestroyIcon, HICON, LR_DEFAULTCOLOR, SM_CXICON,
};
use windows::core::GUID;

use crate::WinError;
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
        }
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
    fn show(&self, notice: &Notice) -> Result<(), WinError> {
        let text = text::notice(notice);
        let mut data = fill(&text, self.icon);
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
        data.dwInfoFlags = info_flags(&text, choice);
        let shown = self.show_or_keep(notice, &data);
        // An icon of an old size goes only once the shell has the new one.
        drop(old);
        shown?;
        *lock(&self.click) = text.click;
        Ok(())
    }
}

impl BalloonNotifier {
    /// The shell call. A refused notice is kept for the next add (`icon_added`). An add that came while the call ran
    /// found nothing to give back, and the icon is there now, so the call is made once more at once.
    fn show_or_keep(&self, notice: &Notice, data: &NOTIFYICONDATAW) -> Result<(), WinError> {
        let adds = lock(&self.kept).adds;
        let mut retried = false;
        loop {
            let error = match (self.shell)(data) {
                Ok(()) => return Ok(()),
                Err(e) => e,
            };
            let mut kept = lock(&self.kept);
            if kept.adds == adds || retried {
                if !kept.notices.contains(notice) {
                    kept.notices.push(notice.clone());
                }
                return Err(error);
            }
            retried = true;
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{OnceLock, Weak};

    use super::*;
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
        fn notifier(&self, choice: IconChoice) -> BalloonNotifier {
            let me = self.clone();
            BalloonNotifier::with_shell(
                ID,
                choice,
                Box::new(move |data| {
                    me.sent.lock().unwrap().push(Sent {
                        flags: data.uFlags,
                        info: data.dwInfoFlags,
                        title: read(&data.szInfoTitle),
                        guid: data.guidItem.to_u128(),
                        balloon_icon: data.hBalloonIcon.0 as isize,
                    });
                    me.fail.lock().unwrap().map_or(Ok(()), Err)
                }),
            )
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
        assert_eq!(shell.sent().len(), 6, "every notice went to the shell");
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
        assert_eq!(shell.sent().len(), 4, "no call of its own at the add");
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

        n.show(&Notice::FirstRun).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(n.icon_added(), [], "it showed, so nothing waits");
        assert_eq!(n.take_click(), NoticeClick::OpenTaskbarSettings);
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
}
