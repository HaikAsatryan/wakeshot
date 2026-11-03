use std::sync::Once;

use windows::{
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{
        ToastDismissedEventArgs, ToastFailedEventArgs, ToastNotification, ToastNotificationManager,
    },
    Win32::System::{
        Registry::{HKEY_CURRENT_USER, REG_SZ, RegDeleteTreeW, RegSetKeyValueW},
        WinRT::{RO_INIT_SINGLETHREADED, RoInitialize},
    },
    core::{HSTRING, PCWSTR, Result},
};

use crate::log;

pub const AUMID_REGISTERED: &str = "Wakeshot.TraySpike";
pub const AUMID_UNREGISTERED: &str = "Wakeshot.TraySpike.Unregistered";

fn aumid_key(aumid: &str) -> HSTRING {
    HSTRING::from(format!("Software\\Classes\\AppUserModelId\\{aumid}"))
}

/// WinRT is loaded only when a toast is sent, so the idle footprint stays Win32 only.
fn init_winrt() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let r = unsafe { RoInitialize(RO_INIT_SINGLETHREADED) };
        log!("RoInitialize(single threaded): {r:?}");
    });
}

/// Per-user registration, no admin: the key unpackaged apps use instead of a Start menu shortcut.
pub fn register(display_name: &str, icon_png: &str) -> String {
    let key = aumid_key(AUMID_REGISTERED);
    let mut out = Vec::new();
    for (name, value) in [("DisplayName", display_name), ("IconUri", icon_png)] {
        let data: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
        let r = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                PCWSTR(HSTRING::from(name).as_ptr()),
                REG_SZ.0,
                Some(data.as_ptr().cast()),
                (data.len() * 2) as u32,
            )
        };
        out.push(format!("{name}={r:?}"));
    }
    out.join(" ")
}

pub fn unregister() -> String {
    let mut out = Vec::new();
    for aumid in [AUMID_REGISTERED, AUMID_UNREGISTERED] {
        let r = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(aumid_key(aumid).as_ptr())) };
        out.push(format!("AppUserModelId\\{aumid}: {r:?}"));
        let settings = HSTRING::from(format!(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Notifications\\Settings\\{aumid}"
        ));
        let r = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(settings.as_ptr())) };
        out.push(format!("Notifications\\Settings\\{aumid}: {r:?}"));
    }
    out.join("; ")
}

/// Shows a toast and logs every step. Errors are logged, never fatal: this is what we want to learn.
pub fn show(aumid: &'static str, title: &str, body: &str) {
    init_winrt();
    match try_show(aumid, title, body) {
        Ok(setting) => log!("toast {aumid}: Show() ok, notifier Setting={setting}"),
        Err(e) => log!("toast {aumid}: error {e}"),
    }
}

fn try_show(aumid: &'static str, title: &str, body: &str) -> Result<String> {
    let xml = XmlDocument::new()?;
    // Silent: the spike tests sound separately.
    xml.LoadXml(&HSTRING::from(format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{title}</text><text>{body}</text></binding></visual><audio silent=\"true\"/></toast>"
    )))?;
    let toast = ToastNotification::CreateToastNotification(&xml)?;
    toast.Dismissed(&TypedEventHandler::<
        ToastNotification,
        ToastDismissedEventArgs,
    >::new(move |_, args| {
        if let Ok(a) = args.ok() {
            log!("toast {aumid}: Dismissed reason={:?}", a.Reason());
        }
        Ok(())
    }))?;
    toast.Failed(
        &TypedEventHandler::<ToastNotification, ToastFailedEventArgs>::new(move |_, args| {
            if let Ok(a) = args.ok() {
                log!("toast {aumid}: Failed error={:?}", a.ErrorCode());
            }
            Ok(())
        }),
    )?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(aumid))?;
    let setting = match notifier.Setting() {
        Ok(s) => format!("{s:?}"),
        Err(e) => format!("error {e}"),
    };
    notifier.Show(&toast)?;
    Ok(setting)
}

/// How many of this app's toasts sit in the notification centre right now.
pub fn history_count(aumid: &str) -> String {
    init_winrt();
    let r = ToastNotificationManager::History()
        .and_then(|h| h.GetHistoryWithId(&HSTRING::from(aumid)))
        .and_then(|v| v.Size());
    match r {
        Ok(n) => n.to_string(),
        Err(e) => format!("error {e}"),
    }
}
